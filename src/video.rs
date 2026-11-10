//! A phone's screen as H.264 (an Annex B byte stream), decoded to frames: the
//! stream is cut at its start codes, and every unit up to the last one whole
//! is decoded, its colours by the matrix and range the stream names.

use image::RgbImage;
use openh264::{decoder::Decoder, formats::YUVSource};

/// Decodes a stream of H.264 into frames.
pub struct H264 {
    decoder: Decoder,
    pending: Vec<u8>,
    /// How the stream's last parameter set says its colours are coded.
    colour: Colour,
}

impl H264 {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            decoder: Decoder::new().map_err(|e| e.to_string())?,
            pending: Vec::new(),
            colour: Colour::default(),
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
            let body = payload(unit);
            if body.first().is_some_and(|header| header & 0x1f == SPS) {
                self.colour = signalled(body);
            }
            if let Some(yuv) = self.decoder.decode(unit).map_err(|e| e.to_string())? {
                let (width, height) = yuv.dimensions();
                let pixels = match (self.colour.matrix.unwrap_or(UNSAID), self.colour.full) {
                    // The crate's own conversion: a stream read so before comes as it did.
                    (Matrix::Bt601, false) => {
                        let mut pixels = vec![0; width * height * 3];
                        yuv.write_rgb8(&mut pixels);
                        pixels
                    }
                    (matrix, full) => rgb(&yuv, matrix, full),
                };
                newest = RgbImage::from_raw(width as u32, height as u32, pixels);
            }
        }
        Ok(newest)
    }
}

/// A unit of Annex B without its start code (nor the zeros a longer start
/// code leaves after the unit before).
pub(crate) fn payload(unit: &[u8]) -> &[u8] {
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

/// Which matrix carries a stream's colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matrix {
    Bt601,
    Bt709,
}

/// How a stream says its colours are coded (its sequence parameter set's VUI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Colour {
    /// `None` where the stream does not say, or names a matrix not read.
    pub matrix: Option<Matrix>,
    /// Whether the samples span 0 to 255 (else 16 to 235).
    pub full: bool,
}

/// What the sequence parameter set `sps` (a unit without its start code) says of its colours;
/// nothing where it holds no such word, or cannot be read.
pub fn signalled(sps: &[u8]) -> Colour {
    vui_colour(&rbsp(sps)).unwrap_or_default()
}

/// A unit's payload without the bytes that keep start codes out of it (`00 00 03` is `00 00`).
fn rbsp(unit: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(unit.len());
    let mut zeros = 0;
    for &byte in unit {
        if zeros >= 2 && byte == 3 {
            zeros = 0;
            continue;
        }
        zeros = if byte == 0 { zeros + 1 } else { 0 };
        out.push(byte);
    }
    out
}

/// The bits of a parameter set, read in order; `None` past its end.
struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<bool> {
        let byte = self.bytes.get(self.at / 8)?;
        let bit = byte >> (7 - self.at % 8) & 1;
        self.at += 1;
        Some(bit == 1)
    }

    fn bits(&mut self, count: u32) -> Option<u32> {
        (0..count).try_fold(0, |n, _| Some(n << 1 | u32::from(self.bit()?)))
    }

    /// An unsigned Exp-Golomb number (`ue(v)`).
    fn ue(&mut self) -> Option<u32> {
        let mut zeros = 0;
        while !self.bit()? {
            zeros += 1;
            if zeros > 31 {
                return None;
            }
        }
        Some((1u32 << zeros) - 1 + self.bits(zeros)?)
    }

    /// A signed one (`se(v)`).
    fn se(&mut self) -> Option<i64> {
        let n = i64::from(self.ue()?);
        Some(if n % 2 == 1 { (n + 1) / 2 } else { -n / 2 })
    }
}

/// The profiles whose parameter sets tell the chroma format and scaling lists (H.264 7.3.2.1.1).
const HIGH_PROFILES: [u32; 13] = [100, 110, 122, 244, 44, 83, 86, 118, 128, 138, 139, 134, 135];

/// The colours a sequence parameter set (its unit's header byte first, `rbsp`) tells in its
/// VUI (H.264 7.3.2.1.1 and E.1.1); `None` where it cannot be read.
fn vui_colour(sps: &[u8]) -> Option<Colour> {
    let mut b = Bits {
        bytes: sps.get(1..)?,
        at: 0,
    };
    let profile = b.bits(8)?;
    b.bits(16)?;
    b.ue()?;
    if HIGH_PROFILES.contains(&profile) {
        let chroma = b.ue()?;
        if chroma == 3 {
            b.bit()?;
        }
        b.ue()?;
        b.ue()?;
        b.bit()?;
        if b.bit()? {
            for list in 0..if chroma == 3 { 12 } else { 8 } {
                if !b.bit()? {
                    continue;
                }
                let (mut last, mut next) = (8i64, 8i64);
                for _ in 0..if list < 6 { 16 } else { 64 } {
                    if next != 0 {
                        next = (last + b.se()?).rem_euclid(256);
                    }
                    if next != 0 {
                        last = next;
                    }
                }
            }
        }
    }
    b.ue()?;
    match b.ue()? {
        0 => {
            b.ue()?;
        }
        1 => {
            b.bit()?;
            b.se()?;
            b.se()?;
            for _ in 0..b.ue()? {
                b.se()?;
            }
        }
        _ => {}
    }
    b.ue()?;
    b.bit()?;
    b.ue()?;
    b.ue()?;
    if !b.bit()? {
        b.bit()?;
    }
    b.bit()?;
    if b.bit()? {
        for _ in 0..4 {
            b.ue()?;
        }
    }
    // The VUI, where there is one: the aspect ratio, the overscan, then the video signal.
    if !b.bit()? {
        return Some(Colour::default());
    }
    if b.bit()? && b.bits(8)? == 255 {
        b.bits(32)?;
    }
    if b.bit()? {
        b.bit()?;
    }
    if !b.bit()? {
        return Some(Colour::default());
    }
    b.bits(3)?;
    let full = b.bit()?;
    let matrix = if b.bit()? {
        b.bits(16)?;
        match b.bits(8)? {
            1 => Some(Matrix::Bt709),
            5 | 6 => Some(Matrix::Bt601),
            _ => None,
        }
    } else {
        None
    };
    Some(Colour { matrix, full })
}

/// The matrix a stream that names none is read with: BT.601, as every stream was read
/// before its word was (a Legion Tab's stream comes back as drawn so, 2026-11-10).
const UNSAID: Matrix = Matrix::Bt601;

/// `Y'CbCr` 4:2:0 as RGB, by `matrix` and range, each chroma sample over its four pixels (as
/// the openh264 crate's own BT.601 conversion does, which the references were cut with).
fn rgb(yuv: &impl YUVSource, matrix: Matrix, full: bool) -> Vec<u8> {
    // In fixed point: 14 bits below the unit.
    const ONE: f64 = 16384.0;
    let (width, height) = yuv.dimensions();
    let (y_stride, u_stride, v_stride) = yuv.strides();
    let (kr, kb) = match matrix {
        Matrix::Bt601 => (0.299, 0.114),
        Matrix::Bt709 => (0.2126, 0.0722),
    };
    let kg = 1.0 - kr - kb;
    let (black, luma_gain, chroma_gain) = if full {
        (0.0, 1.0, 1.0)
    } else {
        (16.0, 255.0 / 219.0, 255.0 / 224.0)
    };
    let fixed = |gain: f64| (gain * ONE).round() as i32;
    let (rv, bu) = (
        fixed(2.0 * (1.0 - kr) * chroma_gain),
        fixed(2.0 * (1.0 - kb) * chroma_gain),
    );
    let (gu, gv) = (
        fixed(-2.0 * (1.0 - kb) * kb / kg * chroma_gain),
        fixed(-2.0 * (1.0 - kr) * kr / kg * chroma_gain),
    );
    // A luma sample's share, rounding to the nearest level included.
    let luma: [i32; 256] =
        std::array::from_fn(|y| fixed((y as f64 - black) * luma_gain) + (ONE / 2.0) as i32);
    let mut out = vec![0; width * height * 3];
    let mut chroma = vec![(0, 0, 0); width.div_ceil(2)];
    for (row, line) in out.chunks_exact_mut(width * 3).enumerate() {
        if row % 2 == 0 {
            let (u, v) = (
                &yuv.u()[row / 2 * u_stride..],
                &yuv.v()[row / 2 * v_stride..],
            );
            for (x, c) in chroma.iter_mut().enumerate() {
                let (u, v) = (i32::from(u[x]) - 128, i32::from(v[x]) - 128);
                *c = (rv * v, gu * u + gv * v, bu * u);
            }
        }
        let y = &yuv.y()[row * y_stride..];
        for (x, pixel) in line.chunks_exact_mut(3).enumerate() {
            let (r, g, b) = chroma[x / 2];
            let level = luma[usize::from(y[x])];
            pixel[0] = ((level + r) >> 14).clamp(0, 255) as u8;
            pixel[1] = ((level + g) >> 14).clamp(0, 255) as u8;
            pixel[2] = ((level + b) >> 14).clamp(0, 255) as u8;
        }
    }
    out
}

/// The type of a sequence parameter set's unit.
const SPS: u8 = 7;

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

    use openh264::encoder::{EncoderConfig, VuiConfig};

    use super::{Colour, H264, Matrix, last_start, payload, rbsp, signalled};

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

    /// The parameter set of a stream encoded with `vui` (none: the encoder writes no colours).
    fn parameter_set(vui: Option<VuiConfig>) -> Vec<u8> {
        let stream = encoded(&YUVBuffer::new(64, 48), vui);
        openh264::nal_units(&stream)
            .map(payload)
            .find(|unit| unit.first().is_some_and(|b| b & 0x1f == 7))
            .expect("a stream starts with its parameter sets")
            .to_vec()
    }

    fn encoded(yuv: &YUVBuffer, vui: Option<VuiConfig>) -> Vec<u8> {
        let config = EncoderConfig::new();
        let config = match vui {
            Some(vui) => config.vui(vui),
            None => config,
        };
        let mut encoder =
            Encoder::with_api_config(openh264::OpenH264API::from_source(), config).unwrap();
        encoder.encode(yuv).unwrap().to_vec()
    }

    #[test]
    fn a_stream_tells_how_its_colours_are_coded() {
        let told = |vui| signalled(&parameter_set(vui));
        assert_eq!(
            told(Some(VuiConfig::bt709())),
            Colour {
                matrix: Some(Matrix::Bt709),
                full: false
            }
        );
        assert_eq!(
            told(Some(VuiConfig::bt709_full())),
            Colour {
                matrix: Some(Matrix::Bt709),
                full: true
            }
        );
        assert_eq!(
            told(Some(VuiConfig::bt601())),
            Colour {
                matrix: Some(Matrix::Bt601),
                full: false
            }
        );
        // An encoder that writes nothing of its colours.
        assert_eq!(told(None), Colour::default());
    }

    /// A parameter set cut short, or of no sense, tells nothing and stops nothing.
    #[test]
    fn a_broken_parameter_set_tells_nothing() {
        let whole = parameter_set(Some(VuiConfig::bt709()));
        for length in 0..whole.len() - 4 {
            assert_eq!(signalled(&whole[..length]), Colour::default(), "{length}");
        }
        for filler in [0x00, 0xff, 0x55] {
            signalled(&[filler; 64]);
        }
    }

    #[test]
    fn the_bytes_that_keep_start_codes_out_are_dropped() {
        assert_eq!(
            rbsp(&[7, 0, 0, 3, 1, 0, 0, 3, 0, 0, 3]),
            [7, 0, 0, 1, 0, 0, 0, 0]
        );
        assert_eq!(rbsp(&[0, 3, 0, 0, 0, 3]), [0, 3, 0, 0, 0]);
    }

    /// A flat picture of `rgb` as a stream of `matrix` and range codes it (the forward
    /// transform, by the matrix's luma weights).
    fn picture(rgb: [u8; 3], matrix: Matrix, full: bool) -> YUVBuffer {
        let (kr, kb) = match matrix {
            Matrix::Bt601 => (0.299, 0.114),
            Matrix::Bt709 => (0.2126, 0.0722),
        };
        let [red, green, blue] = rgb.map(f64::from);
        let luma = kr * red + (1.0 - kr - kb) * green + kb * blue;
        let (cb, cr) = (
            (blue - luma) / (2.0 * (1.0 - kb)),
            (red - luma) / (2.0 * (1.0 - kr)),
        );
        let samples = if full {
            [luma, 128.0 + cb, 128.0 + cr]
        } else {
            [
                16.0 + luma * 219.0 / 255.0,
                128.0 + cb * 224.0 / 255.0,
                128.0 + cr * 224.0 / 255.0,
            ]
        };
        let (width, height) = (64, 48);
        let [luma, cb, cr] = samples.map(|sample| sample.round() as u8);
        let mut yuv = vec![luma; width * height];
        yuv.extend(vec![cb; width * height / 4]);
        yuv.extend(vec![cr; width * height / 4]);
        YUVBuffer::from_vec(yuv, width, height)
    }

    /// The student screen's yellow star and blue badge (as a PC's window shows them) come
    /// back as drawn from a stream of either matrix and range: a Pixel 9 codes its screen
    /// with BT.709, a Legion Tab with BT.601 (2026-11-10; read with BT.601, the Pixel's
    /// yellow came (251, 252, 8)).
    #[test]
    fn colours_come_back_as_the_stream_codes_them() {
        for (vui, matrix, full) in [
            (VuiConfig::bt709(), Matrix::Bt709, false),
            (VuiConfig::bt709_full(), Matrix::Bt709, true),
            (VuiConfig::bt601(), Matrix::Bt601, false),
        ] {
            for drawn in [[255, 238, 0], [15, 136, 188]] {
                let stream = encoded(&picture(drawn, matrix, full), Some(vui));
                let mut video = H264::new().unwrap();
                let frame = video
                    .decode(&stream)
                    .unwrap()
                    .expect("a key frame decodes at once");
                let seen = frame.get_pixel(20, 20).0;
                let off = (0..3).map(|c| seen[c].abs_diff(drawn[c])).max();
                assert!(
                    off <= Some(3),
                    "{matrix:?} full {full}: {seen:?} for {drawn:?}"
                );
            }
        }
    }

    /// Bits written in order, for a parameter set made by hand.
    #[derive(Default)]
    struct Written(Vec<bool>);

    impl Written {
        fn bits(mut self, value: u32, count: u32) -> Self {
            self.0
                .extend((0..count).rev().map(|bit| value >> bit & 1 == 1));
            self
        }

        fn ue(self, value: u32) -> Self {
            let length = 32 - (value + 1).leading_zeros();
            self.bits(0, length - 1).bits(value + 1, length)
        }

        /// The bytes, the last filled up, with the bytes that keep start codes out put in.
        fn unit(mut self) -> Vec<u8> {
            self.0.push(true);
            let mut out: Vec<u8> = Vec::new();
            for byte in self.0.chunks(8) {
                let byte = byte
                    .iter()
                    .enumerate()
                    .fold(0, |n, (i, &bit)| n | u8::from(bit) << (7 - i));
                if out.ends_with(&[0, 0]) && byte <= 3 {
                    out.push(3);
                }
                out.push(byte);
            }
            out
        }
    }

    /// A phone's encoder writes the High profile, whose parameter set tells more before its
    /// colours (the chroma format, scaling lists): 1920 by 1088, BT.709 in the full range.
    #[test]
    fn a_high_profile_stream_tells_its_colours_too() {
        let head = |scaling: bool| {
            let set = Written::default()
                .bits(0x67, 8)
                .bits(100, 8)
                .bits(0, 8)
                .bits(40, 8)
                .ue(0)
                .ue(1)
                .ue(0)
                .ue(0)
                .bits(0, 1)
                .bits(u32::from(scaling), 1);
            if scaling {
                // The first list told (sixteen steps of none), the other seven not.
                set.bits(1, 1).bits(0xffff, 16).bits(0, 7)
            } else {
                set
            }
        };
        let rest = |set: Written| {
            set.ue(0)
                .ue(0)
                .ue(0)
                .ue(1)
                .bits(0, 1)
                .ue(119)
                .ue(67)
                .bits(1, 1)
                .bits(1, 1)
                .bits(0, 1)
                // The VUI: an aspect ratio spelled out (none told: a run of zeros), no
                // overscan, the video signal.
                .bits(1, 1)
                .bits(1, 1)
                .bits(255, 8)
                .bits(0, 16)
                .bits(1, 16)
                .bits(0, 1)
                .bits(1, 1)
                .bits(5, 3)
                .bits(1, 1)
                .bits(1, 1)
                .bits(1, 8)
                .bits(1, 8)
                .bits(1, 8)
                .unit()
        };
        let told = Colour {
            matrix: Some(Matrix::Bt709),
            full: true,
        };
        let plain = rest(head(false));
        // The set holds runs of zeros, kept from reading as a start code.
        assert!(plain.windows(3).any(|w| w == [0, 0, 3]), "{plain:02x?}");
        assert_eq!(signalled(&plain), told);
        assert_eq!(signalled(&rest(head(true))), told);
    }
}
