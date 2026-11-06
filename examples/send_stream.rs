//! Stands in for a phone's streaming app: sends screenshots to Aoi's waiting
//! URL as H.264 over RTMP, each held for a few seconds at 15 frames a second.
//! `cargo run --release --example send_stream -- rtmp://<host>:1935/aoi/<key> <png>...`
//!
//! `CANVAS=2560x1440` boxes each screenshot into a stream of that size, black
//! around it, as Moblin fits a screen into its stream; `UPRIGHT=1` sends it
//! turned upright, as a phone held sideways may; `SECONDS` holds each longer.

use image::{RgbImage, imageops};
use openh264::{
    OpenH264API,
    encoder::{Encoder, EncoderConfig, QpRange},
    formats::{RgbSliceU8, YUVBuffer},
};

/// Frames a second, as the app is asked to send.
const FPS: u32 = 15;

/// `screenshot` as the app would send it: upright if asked, boxed into `canvas`.
fn framed(screenshot: RgbImage, canvas: Option<(u32, u32)>, upright: bool) -> RgbImage {
    let shot = if upright {
        imageops::rotate270(&screenshot)
    } else {
        screenshot
    };
    let Some((width, height)) = canvas else {
        return shot;
    };
    let scale = (width as f32 / shot.width() as f32).min(height as f32 / shot.height() as f32);
    let (w, h) = (
        (shot.width() as f32 * scale) as u32,
        (shot.height() as f32 * scale) as u32,
    );
    let scaled = imageops::resize(&shot, w, h, imageops::FilterType::Triangle);
    let mut out = RgbImage::new(width, height);
    imageops::replace(
        &mut out,
        &scaled,
        i64::from((width - w) / 2),
        i64::from((height - h) / 2),
    );
    out
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .ok_or("usage: send_stream <rtmp-url> <png>...")?;
    let rest = url.strip_prefix("rtmp://").ok_or("not an rtmp:// URL")?;
    let (host, path) = rest.split_once('/').ok_or("no path in the URL")?;
    let key = path.rsplit('/').next().ok_or("no key in the URL")?;
    let canvas = std::env::var("CANVAS").ok().and_then(|c| {
        let (w, h) = c.split_once('x')?;
        Some((w.parse().ok()?, h.parse().ok()?))
    });
    let upright = std::env::var("UPRIGHT").is_ok();
    let seconds: u32 = std::env::var("SECONDS").map_or(3, |s| s.parse().unwrap_or(3));
    // Fine enough for small letters, as a phone's encoder at a high bit rate
    // (measured on 2026-10-14: at a QP of 20 most cells of the item list
    // misread, at 14 or under one or two in 21).
    let config = EncoderConfig::new().qp(QpRange::new(10, 14));
    let mut encoder =
        Encoder::with_api_config(OpenH264API::from_source(), config).map_err(|e| e.to_string())?;
    let mut access_units = Vec::new();
    for path in args {
        let shot = image::open(&path).map_err(|e| e.to_string())?.to_rgb8();
        let frame = framed(shot, canvas, upright);
        let (width, height) = (frame.width() as usize & !1, frame.height() as usize & !1);
        let frame = imageops::crop_imm(&frame, 0, 0, width as u32, height as u32).to_image();
        let yuv = YUVBuffer::from_rgb8_source(RgbSliceU8::new(frame.as_raw(), (width, height)));
        for _ in 0..seconds * FPS {
            access_units.push(encoder.encode(&yuv).map_err(|e| e.to_string())?.to_vec());
        }
        println!("{path}: {width}x{height}");
    }
    let socket = tokio::net::TcpStream::connect(host)
        .await
        .map_err(|e| e.to_string())?;
    // Sent at the app's pace.
    let paced = access_units.into_iter().inspect(|_| {
        std::thread::sleep(std::time::Duration::from_millis(u64::from(1000 / FPS)));
    });
    aoi::broadcast::publish(socket, key, paced).await
}
