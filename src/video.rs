//! A phone's screen as H.264 (an Annex B byte stream), decoded to frames: the
//! stream is cut at its start codes, and every unit up to the last one whole
//! is decoded.

use image::RgbImage;
use openh264::{decoder::Decoder, formats::YUVSource};

/// Decodes a stream of H.264 into frames.
pub struct H264 {
    decoder: Decoder,
    pending: Vec<u8>,
}

impl H264 {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            decoder: Decoder::new().map_err(|e| e.to_string())?,
            pending: Vec::new(),
        })
    }

    /// Takes `bytes` of the stream; the newest frame they complete, if any.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Option<RgbImage>, String> {
        self.pending.extend_from_slice(bytes);
        let Some(cut) = last_start(&self.pending) else {
            return Ok(None);
        };
        let whole: Vec<u8> = self.pending.drain(..cut).collect();
        self.decode(&whole)
    }

    /// Decodes `units`, whole (Annex B); the newest frame they complete, if any.
    pub fn decode(&mut self, units: &[u8]) -> Result<Option<RgbImage>, String> {
        let mut newest = None;
        for unit in openh264::nal_units(units) {
            if let Some(yuv) = self.decoder.decode(unit).map_err(|e| e.to_string())? {
                let (width, height) = yuv.dimensions();
                let mut rgb = vec![0; width * height * 3];
                yuv.write_rgb8(&mut rgb);
                newest = RgbImage::from_raw(width as u32, height as u32, rgb);
            }
        }
        Ok(newest)
    }
}

/// Where the last start code (`00 00 01`, or `00 00 00 01`) in `stream`
/// begins, unless it begins the stream: what comes before it is whole.
fn last_start(stream: &[u8]) -> Option<usize> {
    let at = stream.windows(3).rposition(|w| w == [0, 0, 1])?;
    let at = if at > 0 && stream[at - 1] == 0 {
        at - 1
    } else {
        at
    };
    (at > 0).then_some(at)
}

#[cfg(test)]
mod tests {
    use openh264::{encoder::Encoder, formats::YUVBuffer};

    use super::{H264, last_start};

    #[test]
    fn a_stream_is_whole_up_to_its_last_start_code() {
        assert_eq!(last_start(&[0, 0, 0, 1, 9, 9, 0, 0, 0, 1, 7]), Some(6));
        assert_eq!(last_start(&[0, 0, 0, 1, 9, 9, 0, 0, 1, 7]), Some(6));
        // The first unit alone is not known to be whole.
        assert_eq!(last_start(&[0, 0, 0, 1, 9, 9]), None);
        assert_eq!(last_start(&[9, 9]), None);
    }

    /// A frame encoded, its stream handed over in pieces, comes back once the
    /// next frame's start shows it whole.
    #[test]
    fn frames_come_back_however_the_stream_is_cut() {
        let (width, height) = (64, 48);
        let rgb: Vec<u8> = (0..width * height)
            .flat_map(|i| [(i % 256) as u8, 128, 200])
            .collect();
        let yuv =
            YUVBuffer::from_rgb8_source(openh264::formats::RgbSliceU8::new(&rgb, (width, height)));
        let mut encoder = Encoder::new().unwrap();
        let stream: Vec<u8> = (0..2)
            .flat_map(|_| encoder.encode(&yuv).unwrap().to_vec())
            .collect();
        let mut video = H264::new().unwrap();
        let mut frames = Vec::new();
        for piece in stream.chunks(7) {
            frames.extend(video.push(piece).unwrap());
        }
        let first = frames
            .first()
            .expect("the first frame, the second's start past it");
        assert_eq!(first.dimensions(), (width as u32, height as u32));
        // Near the colour it was drawn in (a lossy codec).
        let p = first.get_pixel(10, 10).0;
        assert!(p[1].abs_diff(128) < 24 && p[2].abs_diff(200) < 24, "{p:?}");
    }
}
