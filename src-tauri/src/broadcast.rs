//! A phone's screen sent by a streaming app (Moblin on an iPhone, or any app
//! that streams over RTMP): while the user waits for it, this computer takes
//! one stream at a time from its local network, its H.264 decoded to frames.

use std::{
    borrow::Cow,
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr},
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use bytes::Bytes;
use image::RgbImage;
use rml_rtmp::{
    handshake::{Handshake, HandshakeProcessResult, PeerType},
    sessions::{
        ClientSession, ClientSessionConfig, ClientSessionEvent, ClientSessionResult,
        PublishRequestType, ServerSession, ServerSessionConfig, ServerSessionEvent,
        ServerSessionResult,
    },
    time::RtmpTimestamp,
};
use serde::Serialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{
    android::{qr_modules, word},
    screen::{self, Turn},
    video::H264,
};

/// Where waiting for a streaming app stands, for the window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Cast {
    /// Not waiting.
    Off,
    /// Waiting for a stream to `url` (its key in it), with QR codes of
    /// Moblin's link to it and of the URL itself (modules row by row, dark
    /// ones true), and why the last stream ended unread, if it did.
    Waiting {
        url: String,
        moblin: Vec<Vec<bool>>,
        plain: Vec<Vec<bool>>,
        problem: Option<String>,
    },
    /// A stream comes in from `from`; `sideways` when the phone's screen came
    /// upright into a wide stream, small (the app would send it larger set to
    /// stream upright).
    Receiving { from: String, sideways: bool },
    /// Waiting could not start: why.
    Failed { message: String },
}

/// The port streams are taken on, RTMP's own.
const PORT: u16 = 1935;
/// How long a wait lasts with nothing sent.
const WAIT: Duration = Duration::from_mins(10);
/// The letters and digits of a wait's key.
const KEY_LENGTH: usize = 10;

type FrameSink = Arc<dyn Fn(RgbImage) + Send + Sync>;
type CastSink = Arc<dyn Fn(&Cast) + Send + Sync>;

/// Waits for a streaming app when the user asks, passing on its frames.
pub struct Receiver {
    frame: FrameSink,
    cast: CastSink,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl Receiver {
    pub fn new(
        frame: impl Fn(RgbImage) + Send + Sync + 'static,
        cast: impl Fn(&Cast) + Send + Sync + 'static,
    ) -> Self {
        Self {
            frame: Arc::new(frame),
            cast: Arc::new(cast),
            task: Mutex::new(None),
        }
    }

    /// Waits under a new key, in place of any wait.
    pub fn start(&self) {
        let (frame, cast) = (self.frame.clone(), self.cast.clone());
        let task = tauri::async_runtime::spawn(async move {
            if let Err(message) = wait(&frame, &cast).await {
                cast(&Cast::Failed { message });
            }
        });
        if let Some(old) = self
            .task
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(task)
        {
            old.abort();
        }
    }

    /// Stops waiting, and any stream coming in.
    pub fn stop(&self) {
        if let Some(task) = self
            .task
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            task.abort();
        }
        (self.cast)(&Cast::Off);
    }
}

/// This computer's address on its local network: the one it would reach
/// beyond it from (nothing is sent).
fn local_ip() -> Option<IpAddr> {
    let socket = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    socket.local_addr().ok().map(|at| at.ip())
}

/// Waits for streams from the local network, one at a time, till none comes
/// for `WAIT`.
async fn wait(frame: &FrameSink, cast: &CastSink) -> Result<(), String> {
    let host = local_ip().ok_or("この PC のネットワークのアドレスが分かりません")?;
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, PORT))
        .await
        .map_err(|e| {
            format!(
                "ポート {PORT} で待ち受けられません（ほかのアプリが使っているかもしれません）: {e}"
            )
        })?;
    let key = word(KEY_LENGTH);
    let url = format!("rtmp://{host}:{PORT}/{APP}/{key}");
    let (moblin, plain) = (qr_modules(&moblin_link(&url))?, qr_modules(&url)?);
    let mut problem = None;
    loop {
        cast(&Cast::Waiting {
            url: url.clone(),
            moblin: moblin.clone(),
            plain: plain.clone(),
            problem: problem.take(),
        });
        let Ok(accepted) = tokio::time::timeout(WAIT, listener.accept()).await else {
            cast(&Cast::Off);
            return Ok(());
        };
        let (socket, peer) = accepted.map_err(text)?;
        if !is_local(peer.ip()) {
            continue;
        }
        let _ = socket.set_nodelay(true);
        let from = peer.ip().to_string();
        let told = |sideways| {
            cast(&Cast::Receiving {
                from: from.clone(),
                sideways,
            });
        };
        problem = receive(socket, &key, &**frame, &told).await.err();
    }
}

/// The application a stream is published to: `rtmp://<host>:<port>/aoi/<key>`.
const APP: &str = "aoi";
/// Why a stream ended unread.
const NOT_H264: &str =
    "H.264 でない映像（H.265 など）は読めません。配信アプリの映像の形式を H.264 にしてください";
const BROKEN: &str = "配信の映像を読み取れませんでした";
const TURNED_AWAY: &str = "キーの違う配信が届いたので断りました。QR コードを読み取り直してください";
const CLOSED: &str = "配信の接続が切れました";
const SILENT: &str = "配信が 30 秒届かなかったので切りました";
/// How long a stream may go silent: an app streams on, over a still screen
/// too, and one that sends nothing would keep the next out.
const SILENCE: Duration = Duration::from_secs(30);

/// Reads what comes over `socket` (none once it closes), silent for no longer
/// than `SILENCE`.
async fn read(socket: &mut TcpStream, buf: &mut [u8]) -> Result<usize, String> {
    tokio::time::timeout(SILENCE, socket.read(buf))
        .await
        .map_err(|_| SILENT.to_owned())?
        .map_err(text)
}

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Runs RTMP's handshake over `socket` as a client or a server; the bytes that
/// came after it.
async fn handshake(
    socket: &mut TcpStream,
    client: bool,
    buf: &mut [u8],
) -> Result<Vec<u8>, String> {
    let mut handshake = Handshake::new(if client {
        PeerType::Client
    } else {
        PeerType::Server
    });
    if client {
        let hello = handshake.generate_outbound_p0_and_p1().map_err(text)?;
        socket.write_all(&hello).await.map_err(text)?;
    }
    loop {
        let n = read(socket, buf).await?;
        if n == 0 {
            return Err(CLOSED.to_owned());
        }
        match handshake.process_bytes(&buf[..n]).map_err(text)? {
            HandshakeProcessResult::InProgress { response_bytes } => {
                socket.write_all(&response_bytes).await.map_err(text)?;
            }
            HandshakeProcessResult::Completed {
                response_bytes,
                remaining_bytes,
            } => {
                socket.write_all(&response_bytes).await.map_err(text)?;
                return Ok(remaining_bytes);
            }
        }
    }
}

/// A stream's video, decoded and set as the game lays it out.
struct Video {
    decoder: H264,
    /// The size of the length before each unit, from the configuration.
    length_size: usize,
    /// How frames that came upright were last turned.
    turn: Option<Turn>,
    /// Whether the screen came upright into a wide stream.
    sideways: bool,
}

impl Video {
    /// Takes a video message; the frame it completes, if any.
    fn take(&mut self, data: &[u8]) -> Result<Option<RgbImage>, String> {
        let units = match tag(data) {
            Tag::Config(record) => {
                let (sets, length_size) = config(record).ok_or(BROKEN)?;
                self.length_size = length_size;
                sets
            }
            Tag::Units(units) => annex_b(units, self.length_size).ok_or(BROKEN)?,
            Tag::Other => return Ok(None),
            Tag::Unsupported => return Err(NOT_H264.to_owned()),
        };
        // A unit the decoder cannot take is passed over: the next key frame
        // mends the picture.
        let Ok(Some(mut image)) = self.decoder.decode(&units) else {
            return Ok(None);
        };
        let trimmed = match screen::trim(&image) {
            Cow::Owned(trimmed) => Some(trimmed),
            Cow::Borrowed(_) => None,
        };
        if let Some(trimmed) = trimmed {
            self.sideways |= image.width() > image.height() && trimmed.height() > trimmed.width();
            image = trimmed;
        }
        if image.height() > image.width() {
            self.turn = screen::turn(&image).or(self.turn);
            if let Some(turn) = self.turn {
                image = turn.apply(&image);
            }
        }
        Ok(Some(image))
    }
}

/// Writes `packet`'s bytes, the server's answer.
async fn answer(socket: &mut TcpStream, result: ServerSessionResult) -> Result<(), String> {
    if let ServerSessionResult::OutboundResponse(packet) = result {
        socket.write_all(&packet.bytes).await.map_err(text)?;
    }
    Ok(())
}

/// Receives what a streaming app publishes over `socket` under `key`, passing
/// on each frame; `told` hears when it starts publishing, and again once the
/// screen comes upright into a wide stream (`true`). It ends when the app
/// stops (or its stream cannot be read: why).
async fn receive(
    mut socket: TcpStream,
    key: &str,
    frame: &(dyn Fn(RgbImage) + Send + Sync),
    told: &(dyn Fn(bool) + Send + Sync),
) -> Result<(), String> {
    let mut buf = vec![0; 1 << 16];
    let mut input = handshake(&mut socket, false, &mut buf).await?;
    let (mut session, results) = ServerSession::new(ServerSessionConfig::new()).map_err(text)?;
    let mut pending: VecDeque<_> = results.into();
    let mut video = Video {
        decoder: H264::new()?,
        length_size: 4,
        turn: None,
        sideways: false,
    };
    loop {
        while let Some(result) = pending.pop_front() {
            let ServerSessionResult::RaisedEvent(event) = result else {
                answer(&mut socket, result).await?;
                continue;
            };
            match event {
                ServerSessionEvent::ConnectionRequested { request_id, .. } => {
                    pending.extend(session.accept_request(request_id).map_err(text)?);
                }
                ServerSessionEvent::PublishStreamRequested {
                    request_id,
                    stream_key,
                    ..
                } if stream_key == key => {
                    pending.extend(session.accept_request(request_id).map_err(text)?);
                    told(false);
                }
                ServerSessionEvent::PublishStreamRequested { request_id, .. } => {
                    let refusal = session
                        .reject_request(request_id, "NetStream.Publish.BadName", "wrong key")
                        .map_err(text)?;
                    for result in refusal {
                        answer(&mut socket, result).await?;
                    }
                    return Err(TURNED_AWAY.to_owned());
                }
                ServerSessionEvent::VideoDataReceived { data, .. } => {
                    let sideways = video.sideways;
                    if let Some(image) = video.take(&data)? {
                        frame(image);
                    }
                    if video.sideways && !sideways {
                        told(true);
                    }
                }
                ServerSessionEvent::PublishStreamFinished { .. } => return Ok(()),
                _ => {}
            }
        }
        if input.is_empty() {
            let n = read(&mut socket, &mut buf).await?;
            if n == 0 {
                return Ok(());
            }
            input.extend_from_slice(&buf[..n]);
        }
        pending.extend(session.handle_input(&input).map_err(text)?);
        input.clear();
    }
}

/// Writes the packets among a client's `results`; the events among them.
async fn send(
    socket: &mut TcpStream,
    results: impl IntoIterator<Item = ClientSessionResult>,
) -> Result<Vec<ClientSessionEvent>, String> {
    let mut events = Vec::new();
    for result in results {
        match result {
            ClientSessionResult::OutboundResponse(packet) => {
                socket.write_all(&packet.bytes).await.map_err(text)?;
            }
            ClientSessionResult::RaisedEvent(event) => events.push(event),
            ClientSessionResult::UnhandleableMessageReceived(_) => {}
        }
    }
    Ok(events)
}

/// Reads what the server sends till `wanted` is among its events.
async fn expect(
    socket: &mut TcpStream,
    session: &mut ClientSession,
    input: &mut Vec<u8>,
    buf: &mut [u8],
    wanted: fn(&ClientSessionEvent) -> bool,
) -> Result<(), String> {
    loop {
        if input.is_empty() {
            let n = read(socket, buf).await?;
            if n == 0 {
                return Err(CLOSED.to_owned());
            }
            input.extend_from_slice(&buf[..n]);
        }
        let results = session.handle_input(input).map_err(text)?;
        input.clear();
        if send(socket, results).await?.iter().any(wanted) {
            return Ok(());
        }
    }
}

/// A streaming app's side: publishes `access_units` (H.264 as an encoder
/// writes it, Annex B) over `socket` under `key`, a fifteenth of a second
/// apart. For the tests, and for the example that stands in for a phone.
pub async fn publish(
    mut socket: TcpStream,
    key: &str,
    access_units: impl IntoIterator<Item = Vec<u8>>,
) -> Result<(), String> {
    let mut buf = vec![0; 1 << 16];
    let mut input = handshake(&mut socket, true, &mut buf).await?;
    let (mut session, results) = ClientSession::new(ClientSessionConfig::new()).map_err(text)?;
    send(&mut socket, results).await?;
    let asked = session.request_connection(APP.to_owned()).map_err(text)?;
    send(&mut socket, [asked]).await?;
    expect(&mut socket, &mut session, &mut input, &mut buf, |e| {
        matches!(e, ClientSessionEvent::ConnectionRequestAccepted)
    })
    .await?;
    let asked = session
        .request_publishing(key.to_owned(), PublishRequestType::Live)
        .map_err(text)?;
    send(&mut socket, [asked]).await?;
    expect(&mut socket, &mut session, &mut input, &mut buf, |e| {
        matches!(e, ClientSessionEvent::PublishRequestAccepted)
    })
    .await?;
    for (n, access_unit) in access_units.into_iter().enumerate() {
        let at = RtmpTimestamp::new(u32::try_from(n).map_err(text)? * 1000 / 15);
        for tag in flv(&access_unit) {
            let sent = session
                .publish_video_data(Bytes::from(tag), at, false)
                .map_err(text)?;
            send(&mut socket, [sent]).await?;
        }
    }
    let stopped = session.stop_publishing().map_err(text)?;
    send(&mut socket, stopped).await?;
    Ok(())
}

/// A unit of Annex B without its start code (nor the zeros a longer start
/// code leaves after the unit before).
fn payload(unit: &[u8]) -> &[u8] {
    let start = unit
        .iter()
        .position(|&b| b != 0)
        .map_or(unit.len(), |at| at + 1);
    let end = unit
        .iter()
        .rposition(|&b| b != 0)
        .map_or(start, |at| at + 1);
    &unit[start..end.max(start)]
}

/// The FLV video tags an encoder's access unit is sent as: the configuration
/// of its parameter sets when it has them, then its other units.
fn flv(access_unit: &[u8]) -> Vec<Vec<u8>> {
    let units: Vec<&[u8]> = openh264::nal_units(access_unit).map(payload).collect();
    let kind = |unit: &[u8]| unit.first().map(|b| b & 0x1f);
    let of = |wanted: u8| units.iter().find(|u| kind(u) == Some(wanted));
    let mut tags = Vec::new();
    if let (Some(sps), Some(pps)) = (of(7), of(8))
        && sps.len() >= 4
    {
        let mut tag = vec![0x17, 0, 0, 0, 0, 1, sps[1], sps[2], sps[3], 0xff, 0xe1];
        for (set, count) in [(sps, None), (pps, Some(1))] {
            tag.extend(count);
            tag.extend(u16::try_from(set.len()).unwrap_or(0).to_be_bytes());
            tag.extend(*set);
        }
        tags.push(tag);
    }
    let picture: Vec<&[u8]> = units
        .iter()
        .copied()
        .filter(|u| kind(u).is_some_and(|k| k != 7 && k != 8))
        .collect();
    if !picture.is_empty() {
        let key = picture.iter().any(|u| kind(u) == Some(5));
        let mut tag = vec![if key { 0x17 } else { 0x27 }, 1, 0, 0, 0];
        for unit in picture {
            tag.extend(u32::try_from(unit.len()).unwrap_or(0).to_be_bytes());
            tag.extend(unit);
        }
        tags.push(tag);
    }
    tags
}

/// What an RTMP video message carries: an FLV video tag.
#[derive(Debug, PartialEq, Eq)]
enum Tag<'a> {
    /// H.264's configuration (an `AVCDecoderConfigurationRecord`).
    Config(&'a [u8]),
    /// H.264's units, each after its length.
    Units(&'a [u8]),
    /// The end of the sequence, or nothing to decode.
    Other,
    /// Video in another codec (H.265 among them).
    Unsupported,
}

fn tag(data: &[u8]) -> Tag<'_> {
    let Some(&head) = data.first() else {
        return Tag::Other;
    };
    // Enhanced RTMP's tags (H.265 and the rest), or a codec other than H.264.
    if head & 0x80 != 0 || head & 0x0f != AVC {
        return Tag::Unsupported;
    }
    match (data.get(1), data.get(5..)) {
        (Some(0), Some(record)) => Tag::Config(record),
        (Some(1), Some(units)) => Tag::Units(units),
        _ => Tag::Other,
    }
}

/// FLV's codec id of H.264.
const AVC: u8 = 7;
/// What starts each unit in Annex B.
const START: [u8; 4] = [0, 0, 0, 1];

/// The parameter sets an `AVCDecoderConfigurationRecord` holds, as Annex B,
/// and the size of the lengths before each unit that follows.
fn config(record: &[u8]) -> Option<(Vec<u8>, usize)> {
    let length_size = usize::from(record.get(4)? & 3) + 1;
    let mut at = 5;
    let mut sets = Vec::new();
    // The SPSs, then the PPSs, each counted before them.
    for mask in [0x1f, 0xff] {
        let count = record.get(at)? & mask;
        at += 1;
        for _ in 0..count {
            let length = usize::from(u16::from_be_bytes([*record.get(at)?, *record.get(at + 1)?]));
            sets.extend(START);
            sets.extend(record.get(at + 2..at + 2 + length)?);
            at += 2 + length;
        }
    }
    Some((sets, length_size))
}

/// Units each after a `length_size`-byte length, as Annex B.
fn annex_b(units: &[u8], length_size: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(units.len());
    let mut rest = units;
    while !rest.is_empty() {
        let (length, after) = rest.split_at_checked(length_size)?;
        let length = length.iter().fold(0, |n, &b| n << 8 | usize::from(b));
        let (unit, after) = after.split_at_checked(length)?;
        out.extend(START);
        out.extend(unit);
        rest = after;
    }
    Some(out)
}

/// The link that adds the stream at `url` to Moblin, set for reading.
pub fn moblin_link(url: &str) -> String {
    let settings = serde_json::json!({
        "streams": [{
            "name": "Aoi",
            "url": url,
            "selected": true,
            "backgroundStreaming": true,
            "video": {
                "codec": "H.264/AVC",
                "resolution": MOBLIN_RESOLUTION,
                "fps": MOBLIN_FPS,
                "bitrate": MOBLIN_BIT_RATE,
            },
        }],
    });
    let encoded: String = settings
        .to_string()
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    format!("moblin://?{encoded}")
}

/// What Moblin streams at for reading: its largest 16:9 short of 4K (a phone's
/// screen scaled down loses the letters: measured on 2026-10-14 with a Pixel
/// 9's frames at 2424x1080, fitted into 1920x1080 three cells in 21 of the
/// item list misread, into 2560x1440 none), its fewest frames a second, and
/// its highest bit rate.
const MOBLIN_RESOLUTION: &str = "2560x1440";
const MOBLIN_FPS: u32 = 15;
const MOBLIN_BIT_RATE: u32 = 50_000_000;

/// Whether `ip` is on this computer's local network (or this computer).
pub fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => match ip.to_ipv4_mapped() {
            Some(ip) => is_local(IpAddr::V4(ip)),
            None => ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::IpAddr,
        sync::{Arc, Mutex},
    };

    use image::RgbImage;
    use openh264::{
        encoder::Encoder,
        formats::{RgbSliceU8, YUVBuffer},
    };
    use tokio::net::{TcpListener, TcpStream};

    use super::{Tag, annex_b, config, is_local, moblin_link, publish, receive, tag};

    /// Three frames of 64x48 as a phone's encoder writes them, the screen
    /// `shown` columns wide in their middle, black around it.
    fn encoded(shown: usize) -> Vec<Vec<u8>> {
        let (width, height): (usize, usize) = (64, 48);
        let mut encoder = Encoder::new().unwrap();
        (0..3)
            .map(|n| {
                let rgb: Vec<u8> = (0..width * height)
                    .flat_map(|i| {
                        if (i % width).abs_diff(width / 2) * 2 < shown {
                            [((i + n) % 200) as u8 + 40, 128, 200]
                        } else {
                            [0; 3]
                        }
                    })
                    .collect();
                let yuv = YUVBuffer::from_rgb8_source(RgbSliceU8::new(&rgb, (width, height)));
                encoder.encode(&yuv).unwrap().to_vec()
            })
            .collect()
    }

    /// What a receiver waiting under `wanted` passes on of a stream of
    /// `shown` columns published under `key`: the frames, what it told as it
    /// received (whether the screen came upright into a wide stream), and how
    /// the receiver and the sender end.
    async fn stream(
        key: &str,
        wanted: &str,
        shown: usize,
    ) -> (
        Vec<RgbImage>,
        Vec<bool>,
        Result<(), String>,
        Result<(), String>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let at = listener.local_addr().unwrap();
        let key = key.to_owned();
        let sending = tokio::spawn(async move {
            let socket = TcpStream::connect(at).await.unwrap();
            publish(socket, &key, encoded(shown)).await
        });
        let (socket, _) = listener.accept().await.unwrap();
        let frames = Arc::new(Mutex::new(Vec::new()));
        let told = Arc::new(Mutex::new(Vec::new()));
        let received = {
            let (frames, told) = (frames.clone(), told.clone());
            receive(
                socket,
                wanted,
                &move |image| frames.lock().unwrap().push(image),
                &move |sideways| told.lock().unwrap().push(sideways),
            )
            .await
        };
        let sent = sending.await.unwrap();
        let frames = frames.lock().unwrap().clone();
        let told = told.lock().unwrap().clone();
        (frames, told, received, sent)
    }

    #[tokio::test]
    async fn a_streaming_apps_frames_come_through() {
        let (frames, told, received, sent) = stream("Ab3", "Ab3", 64).await;
        assert_eq!(received, Ok(()));
        assert_eq!(sent, Ok(()));
        assert_eq!(told, [false]);
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].dimensions(), (64, 48));
    }

    /// A phone held sideways may send its screen upright into a wide stream,
    /// too small to read: the receiver tells, and passes on the screen alone.
    #[tokio::test]
    async fn an_upright_screen_in_a_wide_stream_is_told() {
        let (frames, told, received, _) = stream("Ab3", "Ab3", 16).await;
        assert_eq!(received, Ok(()));
        assert_eq!(told, [false, true]);
        let (width, height) = frames[0].dimensions();
        assert!(width < height && height == 48, "{width}x{height}");
    }

    #[tokio::test]
    async fn a_stream_under_another_key_is_turned_away() {
        let (frames, told, received, sent) = stream("guess", "Ab3", 64).await;
        assert!(received.is_err());
        assert!(sent.is_err());
        assert!(told.is_empty());
        assert!(frames.is_empty());
    }

    const START: [u8; 4] = [0, 0, 0, 1];

    /// An `AVCDecoderConfigurationRecord` of one SPS and one PPS, lengths of 4 bytes.
    fn record(sps: &[u8], pps: &[u8]) -> Vec<u8> {
        let mut record = vec![1, 0x64, 0, 0x28, 0xff, 0xe1];
        record.extend((sps.len() as u16).to_be_bytes());
        record.extend(sps);
        record.push(1);
        record.extend((pps.len() as u16).to_be_bytes());
        record.extend(pps);
        record
    }

    #[test]
    fn a_video_tag_tells_what_it_carries() {
        let record = record(&[0x67, 1, 2], &[0x68, 3]);
        let mut sequence = vec![0x17, 0, 0, 0, 0];
        sequence.extend(&record);
        assert_eq!(tag(&sequence), Tag::Config(&record));
        let units = [0x27, 1, 0, 0, 0, 0, 0, 0, 1, 0x41];
        assert_eq!(tag(&units), Tag::Units(&units[5..]));
        assert_eq!(tag(&[0x17, 2, 0, 0, 0]), Tag::Other);
        // H.265, legacy and enhanced (`hvc1`).
        assert_eq!(tag(&[0x1c, 1, 0, 0, 0]), Tag::Unsupported);
        assert_eq!(tag(&[0x91, b'h', b'v', b'c', b'1']), Tag::Unsupported);
        assert_eq!(tag(&[0x17]), Tag::Other);
    }

    #[test]
    fn h264_comes_out_with_start_codes() {
        let (sets, length_size) = config(&record(&[0x67, 1, 2], &[0x68, 3])).unwrap();
        assert_eq!(
            sets,
            [&START[..], &[0x67, 1, 2], &START, &[0x68, 3]].concat()
        );
        assert_eq!(length_size, 4);
        let units = [0, 0, 0, 3, 0x41, 9, 9, 0, 0, 0, 2, 0x41, 8];
        assert_eq!(
            annex_b(&units, 4).unwrap(),
            [&START[..], &[0x41, 9, 9], &START, &[0x41, 8]].concat()
        );
        // Cut short.
        assert_eq!(annex_b(&units[..10], 4), None);
        assert_eq!(config(&record(&[0x67, 1, 2], &[0x68, 3])[..8]), None);
    }

    /// Percent-decodes `text`.
    fn decode(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                out.push(u8::from_str_radix(&text[i + 1..i + 3], 16).unwrap());
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        String::from_utf8(out).unwrap()
    }

    /// Moblin takes the stream with H.264 (it would send H.265), as large and
    /// as clear as it streams, few frames a second, and streaming on while the
    /// game is in front.
    #[test]
    fn the_moblin_link_adds_the_stream_set_for_reading() {
        let url = "rtmp://192.168.1.20:1935/aoi/Ab3";
        let link = moblin_link(url);
        let json = link.strip_prefix("moblin://?").unwrap();
        assert!(
            json.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~%".contains(&b))
        );
        let settings: serde_json::Value = serde_json::from_str(&decode(json)).unwrap();
        let stream = &settings["streams"][0];
        assert_eq!(stream["url"], url);
        assert_eq!(stream["selected"], true);
        assert_eq!(stream["backgroundStreaming"], true);
        let video = &stream["video"];
        assert_eq!(video["codec"], "H.264/AVC");
        assert_eq!(video["resolution"], "2560x1440");
        assert_eq!(video["fps"], 15);
        assert_eq!(video["bitrate"], 50_000_000);
    }

    #[test]
    fn only_the_local_network_may_send() {
        for ip in [
            "192.168.1.20",
            "10.0.0.5",
            "172.16.3.4",
            "127.0.0.1",
            "169.254.1.1",
            "fe80::1",
            "fd00::5",
            "::1",
            "::ffff:192.168.1.20",
        ] {
            assert!(is_local(ip.parse::<IpAddr>().unwrap()), "{ip}");
        }
        for ip in [
            "8.8.8.8",
            "172.32.0.1",
            "100.64.0.1",
            "2001:db8::1",
            "::ffff:8.8.8.8",
        ] {
            assert!(!is_local(ip.parse::<IpAddr>().unwrap()), "{ip}");
        }
    }
}
