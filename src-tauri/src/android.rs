//! A phone's screen over USB or Wi-Fi, with no `adb` installed: USB debugging
//! allowed on the phone, or Android 11's wireless debugging paired by a QR code
//! as Android Studio pairs; then the screen recorded as raw frames
//! (`screenrecord --output-format=frames`).

use std::{
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use image::RgbImage;
use libadb::{
    AdbKey, Connection, Feature, TlsClientConfig, TlsIdentity, TokioTcp, exec,
    keys::rsa::rand_core::{OsRng, RngCore},
    pairing::pair,
    transport::{nusb::UsbTransport, tls::MaybeTls},
};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::{Deserialize, Serialize};

/// The text of the QR code a phone scans to pair: the service `name` it
/// announces back, and the `password` the pairing runs on.
pub fn pairing_qr(name: &str, password: &str) -> String {
    format!("WIFI:T:ADB;S:{name};P:{password};;")
}

/// The pixel format of a raw frame: `HAL_PIXEL_FORMAT_RGB_888`.
const RGB_888: u32 = 3;
/// A raw frame's header: five little-endian words.
const HEADER: usize = 20;

/// Raw frames cut from `screenrecord --output-format=frames`: each a header
/// of five little-endian words (the length of what follows, the width, the
/// height, the row's bytes, the pixel format), then its RGB rows.
#[derive(Default)]
pub struct Frames {
    pending: Vec<u8>,
}

impl Frames {
    /// Takes `bytes` of the stream; the newest frame they complete, if any.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Option<RgbImage>, String> {
        self.pending.extend_from_slice(bytes);
        let mut newest = None;
        let mut at = 0;
        while self.pending.len() - at >= HEADER {
            let word = |i: usize| {
                let b = &self.pending[at + 4 * i..at + 4 * i + 4];
                u32::from_le_bytes([b[0], b[1], b[2], b[3]])
            };
            let (length, width, height, stride, format) =
                (word(0), word(1), word(2), word(3), word(4));
            let data = width as usize * height as usize * 3;
            if format != RGB_888 || stride != width * 3 || length as usize != HEADER - 4 + data {
                return Err(format!(
                    "unknown frame {width}x{height} stride {stride} format {format}"
                ));
            }
            if self.pending.len() - at < HEADER + data {
                break;
            }
            newest = Some((at + HEADER, width, height));
            at += HEADER + data;
        }
        let frame = newest.map(|(from, width, height)| {
            let rgb = self.pending[from..from + width as usize * height as usize * 3].to_vec();
            RgbImage::from_raw(width, height, rgb).expect("as many bytes as the header says")
        });
        self.pending.drain(..at);
        Ok(frame)
    }
}

/// The phone's model in its banner (`device::ro.product.model=Pixel 9;...`).
pub fn model(banner: &[u8]) -> Option<String> {
    let banner = String::from_utf8_lossy(banner);
    let (_, rest) = banner.split_once("ro.product.model=")?;
    rest.split(';').next().map(str::to_owned)
}

/// The instance an mDNS service's full name names
/// ("adb-XXX._adb-tls-connect._tcp.local." is "adb-XXX").
pub fn instance<'a>(fullname: &'a str, service: &str) -> &'a str {
    fullname
        .strip_suffix(service)
        .and_then(|name| name.strip_suffix('.'))
        .unwrap_or(fullname)
}

/// The services a phone announces over mDNS: to pair, and to connect once paired.
const PAIRING: &str = "_adb-tls-pairing._tcp.local.";
const CONNECT: &str = "_adb-tls-connect._tcp.local.";
/// How long a QR code waits to be scanned.
const PAIRING_WAIT: Duration = Duration::from_mins(10);
/// How long the paired phone is looked for over Wi-Fi at a time, and the pause
/// between tries (a USB cable is looked for between them).
const LOOK: Duration = Duration::from_secs(3);
const RETRY: Duration = Duration::from_secs(3);
/// How long a USB connection may take before the phone is taken to be asking
/// its user to allow it.
const ASKING: Duration = Duration::from_secs(2);
/// From Android 14 (API 34) the screen records without end; before, for three
/// minutes at a time.
const ENDLESS_FROM: u32 = 34;
/// The key's name, which the phone lists among its paired devices.
const KEY_NAME: &str = "Aoi";
/// The file the linked phone is kept in, and the key's.
const PAIRED: &str = "android.json";
const KEY: &str = "adbkey.pem";
/// ADB's USB interface: vendor class, subclass and protocol.
const ADB_INTERFACE: (u8, u8, u8) = (0xff, 0x42, 0x01);

/// How a phone's screen comes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Via {
    Usb,
    Wifi,
}

/// Where the phone link stands, for the window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Link {
    /// No phone has been linked.
    Unpaired,
    /// A QR code waits to be scanned (its modules row by row, dark ones true);
    /// a phone plugged in by USB is taken too.
    Pairing { qr: Vec<Vec<bool>> },
    /// `device`, linked, is looked for over USB and Wi-Fi.
    Searching { device: String },
    /// A phone plugged in by USB asks its user to allow this computer.
    Allowing,
    /// The screen of `device` comes in, `via` USB or Wi-Fi.
    Streaming { device: String, via: Via },
    /// `device`, linked, left unconnected by the user.
    Stopped { device: String },
    /// Linking did not go through: why.
    Failed { message: String },
}

/// The phone linked, kept beside the key: the instance it announces its
/// Wi-Fi connect port under once paired (none for a phone only plugged in),
/// its model, and whether to connect at start.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Paired {
    guid: Option<String>,
    device: String,
    connect: bool,
}

type FrameSink = Arc<dyn Fn(RgbImage) + Send + Sync>;
type LinkSink = Arc<dyn Fn(&Link) + Send + Sync>;

/// What a task of the link works with.
struct Tools {
    key: AdbKey,
    tls: TlsClientConfig,
    mdns: ServiceDaemon,
    dir: PathBuf,
    frame: FrameSink,
    link: LinkSink,
    paused: Arc<AtomicBool>,
}

/// The link to a phone: pairs it or takes it by USB, keeps it connected and
/// passes on its frames.
pub struct Phone {
    /// Where the key and the phone linked are kept.
    dir: PathBuf,
    frame: FrameSink,
    link: LinkSink,
    /// While set, no frames are received (the reading is paused).
    paused: Arc<AtomicBool>,
    mdns: OnceLock<Result<ServiceDaemon, String>>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl Phone {
    pub fn new(
        dir: PathBuf,
        paused: Arc<AtomicBool>,
        frame: impl Fn(RgbImage) + Send + Sync + 'static,
        link: impl Fn(&Link) + Send + Sync + 'static,
    ) -> Self {
        Self {
            dir,
            frame: Arc::new(frame),
            link: Arc::new(link),
            paused,
            mdns: OnceLock::new(),
            task: Mutex::new(None),
        }
    }

    /// Tells where the link stands, and connects to the phone linked if the
    /// user left it connected.
    pub fn start(&self) {
        match self.paired() {
            Some(paired) if paired.connect => self.keep_connected(paired),
            Some(paired) => (self.link)(&Link::Stopped {
                device: paired.device,
            }),
            None => (self.link)(&Link::Unpaired),
        }
    }

    /// Shows a QR code to pair a phone over Wi-Fi with, and takes a phone
    /// plugged in by USB meanwhile; then keeps whichever came connected.
    pub fn pair(&self) {
        let (name, password) = (format!("aoi-{}", word(6)), word(10));
        let code = match qrcode::QrCode::new(pairing_qr(&name, &password)) {
            Ok(code) => code,
            Err(e) => {
                return (self.link)(&Link::Failed {
                    message: e.to_string(),
                });
            }
        };
        let qr = code
            .to_colors()
            .chunks(code.width())
            .map(|row| row.iter().map(|&c| c == qrcode::Color::Dark).collect())
            .collect();
        let Some(tools) = self.tools() else {
            return;
        };
        (self.link)(&Link::Pairing { qr });
        self.run(async move {
            let guid = tokio::select! {
                paired = pair_phone(&tools, &name, &password) => match paired {
                    Ok(guid) => Some(guid),
                    Err(message) => return (tools.link)(&Link::Failed { message }),
                },
                () = plugged() => None,
            };
            let paired = Paired {
                guid,
                device: "Android".to_owned(),
                connect: true,
            };
            let _ = save(&tools.dir, &paired);
            keep(tools, paired).await;
        });
    }

    /// Connects to the phone linked, now and at the next start.
    pub fn connect(&self) {
        if let Some(mut paired) = self.paired() {
            paired.connect = true;
            let _ = save(&self.dir, &paired);
            self.keep_connected(paired);
        }
    }

    /// Leaves the phone linked unconnected, now and at the next start.
    pub fn stop(&self) {
        self.cancel();
        match self.paired() {
            Some(mut paired) => {
                paired.connect = false;
                let _ = save(&self.dir, &paired);
                (self.link)(&Link::Stopped {
                    device: paired.device,
                });
            }
            None => (self.link)(&Link::Unpaired),
        }
    }

    fn keep_connected(&self, paired: Paired) {
        if let Some(tools) = self.tools() {
            self.run(keep(tools, paired));
        }
    }

    /// What a task works with; `None` once why it cannot be had is told.
    fn tools(&self) -> Option<Tools> {
        let made = (|| {
            let key = key(&self.dir)?;
            let identity = TlsIdentity::from_key(&key, &mut OsRng).map_err(|e| e.to_string())?;
            let tls = TlsClientConfig::adb(&identity).map_err(|e| e.to_string())?;
            let mdns = self
                .mdns
                .get_or_init(|| ServiceDaemon::new().map_err(|e| e.to_string()))
                .clone()?;
            Ok::<_, String>(Tools {
                key,
                tls,
                mdns,
                dir: self.dir.clone(),
                frame: self.frame.clone(),
                link: self.link.clone(),
                paused: self.paused.clone(),
            })
        })();
        made.map_err(|message| (self.link)(&Link::Failed { message }))
            .ok()
    }

    /// Runs `task` in place of the one running.
    fn run(&self, task: impl Future<Output = ()> + Send + 'static) {
        self.cancel();
        *self.task.lock().unwrap_or_else(PoisonError::into_inner) =
            Some(tauri::async_runtime::spawn(task));
    }

    fn cancel(&self) {
        let task = self
            .task
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(task) = task {
            task.abort();
        }
    }

    fn paired(&self) -> Option<Paired> {
        let text = std::fs::read_to_string(self.dir.join(PAIRED)).ok()?;
        serde_json::from_str(&text).ok()
    }
}

fn save(dir: &Path, paired: &Paired) -> Result<(), String> {
    let text = serde_json::to_string_pretty(paired).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(PAIRED), text).map_err(|e| e.to_string())
}

/// The key kept in `dir`, made the first time.
fn key(dir: &Path) -> Result<AdbKey, String> {
    let path = dir.join(KEY);
    if let Ok(pem) = std::fs::read_to_string(&path) {
        return AdbKey::from_pkcs8_pem(&pem, &mut OsRng, KEY_NAME).map_err(|e| e.to_string());
    }
    let key = AdbKey::generate(&mut OsRng, KEY_NAME).map_err(|e| e.to_string())?;
    let pem = key.to_pkcs8_pem().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(&path, pem.as_bytes()).map_err(|e| e.to_string())?;
    Ok(key)
}

/// Letters and digits, `n` of them, drawn at random.
fn word(n: usize) -> String {
    const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    (0..n)
        .map(|_| char::from(LETTERS[OsRng.next_u32() as usize % LETTERS.len()]))
        .collect()
}

/// The ADB interface of a phone plugged in by USB, claimed. A device that is
/// no composite lists no interfaces on Windows, so its configuration is read.
fn usb() -> Option<UsbTransport> {
    nusb::list_devices().ok()?.find_map(|info| {
        let listed = info.interfaces().count() > 0;
        let adb = info
            .interfaces()
            .any(|i| (i.class(), i.subclass(), i.protocol()) == ADB_INTERFACE);
        if listed && !adb {
            return None;
        }
        let device = info.open().ok()?;
        let config = device.active_configuration().ok()?;
        let number = config.interfaces().find_map(|group| {
            group
                .alt_settings()
                .any(|a| (a.class(), a.subclass(), a.protocol()) == ADB_INTERFACE)
                .then(|| group.interface_number())
        })?;
        UsbTransport::new(device.claim_interface(number).ok()?)
    })
}

/// Waits for a phone to be plugged in by USB.
async fn plugged() {
    // Claimed to look, and let go at once: `keep` claims it again.
    while usb().is_none() {
        tokio::time::sleep(RETRY).await;
    }
}

/// The address and port of the instance of `service` named `wanted`, if it
/// is announced within `within`.
async fn find(
    mdns: &ServiceDaemon,
    service: &str,
    wanted: &str,
    within: Duration,
) -> Option<(IpAddr, u16)> {
    let _ = mdns.stop_browse(service);
    let events = mdns.browse(service).ok()?;
    let found = tokio::time::timeout(within, async {
        while let Ok(event) = events.recv_async().await {
            if let ServiceEvent::ServiceResolved(info) = event
                && instance(info.get_fullname(), service) == wanted
            {
                let addresses: Vec<IpAddr> = info
                    .get_addresses()
                    .iter()
                    .map(mdns_sd::ScopedIp::to_ip_addr)
                    .collect();
                let address = addresses.iter().find(|a| a.is_ipv4()).or(addresses.first());
                if let Some(&address) = address {
                    return Some((address, info.get_port()));
                }
            }
        }
        None
    })
    .await;
    let _ = mdns.stop_browse(service);
    found.ok().flatten()
}

/// Pairs the phone that scanned the QR code of `name` and `password`; its guid.
async fn pair_phone(tools: &Tools, name: &str, password: &str) -> Result<String, String> {
    let (address, port) = find(&tools.mdns, PAIRING, name, PAIRING_WAIT)
        .await
        .ok_or("QR コードが読み取られないまま時間が過ぎました")?;
    let tcp = tokio::net::TcpStream::connect((address, port))
        .await
        .map_err(|e| format!("スマホにつながりませんでした: {e}"))?;
    let _ = tcp.set_nodelay(true);
    let mut transport = MaybeTls::plain(TokioTcp::new(tcp));
    let paired = pair(&mut transport, &tools.tls, &tools.key, password, &mut OsRng)
        .await
        .map_err(|e| format!("ペア設定できませんでした: {e}"))?;
    Ok(paired.guid)
}

/// Keeps the phone linked connected and its frames coming: over USB while it
/// is plugged in, else over Wi-Fi once paired. It is looked for again whenever
/// it drops (unplugged, wireless debugging turned off); no frames are taken
/// while the reading is paused.
async fn keep(mut tools: Tools, mut paired: Paired) {
    loop {
        while tools.paused.load(Ordering::Relaxed) {
            tokio::time::sleep(RETRY).await;
        }
        (tools.link)(&Link::Searching {
            device: paired.device.clone(),
        });
        let streamed = if let Some(transport) = usb() {
            stream_usb(&mut tools, &mut paired, transport).await
        } else if let Some(guid) = paired.guid.clone()
            && let Some(at) = find(&tools.mdns, CONNECT, &guid, LOOK).await
        {
            stream_wifi(&mut tools, &mut paired, at).await
        } else {
            Err(String::new())
        };
        if streamed.is_err() {
            tokio::time::sleep(RETRY).await;
        }
    }
}

/// Receives the screen of the phone plugged in at `transport`. A phone that
/// has not allowed the key asks its user, which takes as long as it takes.
async fn stream_usb(
    tools: &mut Tools,
    paired: &mut Paired,
    transport: UsbTransport,
) -> Result<(), String> {
    let conn = {
        let connecting = Connection::<_>::connect(transport, &mut tools.key, &[Feature::ShellV2]);
        tokio::pin!(connecting);
        if let Ok(conn) = tokio::time::timeout(ASKING, &mut connecting).await {
            conn
        } else {
            (tools.link)(&Link::Allowing);
            connecting.await
        }
    };
    let mut conn = conn.map_err(|e| e.to_string())?;
    record(tools, paired, &mut conn, Via::Usb).await
}

/// Receives the screen of the phone paired at `at` over Wi-Fi.
async fn stream_wifi(
    tools: &mut Tools,
    paired: &mut Paired,
    at: (IpAddr, u16),
) -> Result<(), String> {
    let tcp = tokio::net::TcpStream::connect(at)
        .await
        .map_err(|e| e.to_string())?;
    let _ = tcp.set_nodelay(true);
    let transport = MaybeTls::plain(TokioTcp::new(tcp));
    let mut conn =
        Connection::<_>::connect_tls(transport, &mut tools.key, &[Feature::ShellV2], &tools.tls)
            .await
            .map_err(|e| e.to_string())?;
    record(tools, paired, &mut conn, Via::Wifi).await
}

/// Records the screen of the phone at `conn`, passing on each frame, till the
/// recording ends or the reading pauses. The phone's model, kept, and the link
/// streaming are told once its first frame comes.
async fn record<T>(
    tools: &Tools,
    paired: &mut Paired,
    conn: &mut Connection<T>,
    via: Via,
) -> Result<(), String>
where
    T: embedded_io_async::Read + embedded_io_async::Write,
    T::Error: std::fmt::Display,
{
    let device = conn
        .device_banner()
        .and_then(model)
        .unwrap_or_else(|| paired.device.clone());
    let mut buf = vec![0u8; 1 << 20];
    let n = exec::run(conn, "getprop ro.build.version.sdk", &mut buf)
        .await
        .map_err(|e| e.to_string())?;
    let sdk: u32 = String::from_utf8_lossy(&buf[..n])
        .trim()
        .parse()
        .unwrap_or(0);
    let limit = if sdk >= ENDLESS_FROM { 0 } else { 180 };
    let command = format!("screenrecord --output-format=frames --time-limit {limit} -");
    let mut destination = [0u8; 128];
    let mut recording = exec::open(conn, &command, &mut destination)
        .await
        .map_err(|e| e.to_string())?;
    let mut frames = Frames::default();
    let mut first = true;
    while !tools.paused.load(Ordering::Relaxed) {
        let n = match recording.read(&mut buf).await {
            Ok(n) => n,
            Err(libadb::Error::ChannelClosed) => return Ok(()),
            Err(e) => return Err(e.to_string()),
        };
        if let Some(image) = frames.push(&buf[..n])? {
            if first {
                first = false;
                if paired.device != device {
                    device.clone_into(&mut paired.device);
                    let _ = save(&tools.dir, paired);
                }
                (tools.link)(&Link::Streaming {
                    device: device.clone(),
                    via,
                });
            }
            (tools.frame)(image);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Frames, instance, model, pairing_qr};

    /// A frame of `width` x `height` whose every byte is `fill`.
    fn frame(width: u32, height: u32, fill: u8) -> Vec<u8> {
        let data = (width * height * 3) as usize;
        let mut out = Vec::new();
        for word in [16 + data as u32, width, height, width * 3, 3] {
            out.extend(word.to_le_bytes());
        }
        out.extend(std::iter::repeat_n(fill, data));
        out
    }

    #[test]
    fn the_pairing_qr_carries_the_name_and_the_password() {
        assert_eq!(pairing_qr("aoi-x1", "Ab3"), "WIFI:T:ADB;S:aoi-x1;P:Ab3;;");
    }

    #[test]
    fn frames_are_cut_from_the_stream_however_it_arrives() {
        let stream = [frame(2, 1, 7), frame(2, 1, 9), frame(3, 2, 5)].concat();
        let mut frames = Frames::default();
        // A frame split anywhere comes once it is whole.
        assert_eq!(frames.push(&stream[..10]).unwrap(), None);
        let first = frames.push(&stream[10..30]).unwrap().unwrap();
        assert_eq!((first.dimensions(), first.as_raw()[0]), ((2, 1), 7));
        // Two whole frames at once: the newer one.
        let rest = frames.push(&stream[30..]).unwrap().unwrap();
        assert_eq!((rest.dimensions(), rest.as_raw()[0]), ((3, 2), 5));
        assert_eq!(frames.push(&[]).unwrap(), None);
    }

    #[test]
    fn a_frame_of_another_pixel_format_is_refused() {
        let mut other = frame(2, 1, 0);
        other[16] = 1;
        assert!(Frames::default().push(&other).is_err());
    }

    #[test]
    fn the_model_is_read_from_the_banner() {
        let banner = b"device::ro.product.name=tokay;ro.product.model=Pixel 9;ro.product.device=tokay;features=shell_v2";
        assert_eq!(model(banner).as_deref(), Some("Pixel 9"));
        assert_eq!(model(b"device::features=shell_v2"), None);
    }

    #[test]
    fn an_instance_is_its_full_name_short_of_the_service() {
        let full = "adb-47050DLAQ000XJ-4OU0sd._adb-tls-connect._tcp.local.";
        assert_eq!(
            instance(full, "_adb-tls-connect._tcp.local."),
            "adb-47050DLAQ000XJ-4OU0sd"
        );
    }
}
