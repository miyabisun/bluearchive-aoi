//! Japanese text recognition with the OCR built into Windows (`Windows.Media.Ocr`).

use image::RgbImage;
use windows::{
    Globalization::Language,
    Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::DataWriter,
    core::HSTRING,
};

pub struct Ocr(OcrEngine);

impl Ocr {
    /// The engine for Japanese, which Windows offers when its Japanese language
    /// features are installed (they are on a Japanese Windows).
    pub fn japanese() -> Result<Self, String> {
        let language = Language::CreateLanguage(&HSTRING::from("ja")).map_err(|e| e.to_string())?;
        OcrEngine::TryCreateFromLanguage(&language)
            .map(Self)
            .map_err(|e| format!("Japanese OCR is not available: {e}"))
    }

    /// The text in `image`, its lines joined by "/".
    pub fn read(&self, image: &RgbImage) -> Result<String, String> {
        let fail = |e: windows::core::Error| e.to_string();
        let (width, height) = image.dimensions();
        let bgra: Vec<u8> = image
            .pixels()
            .flat_map(|p| [p[2], p[1], p[0], 255])
            .collect();
        let writer = DataWriter::new().map_err(fail)?;
        writer.WriteBytes(&bgra).map_err(fail)?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
            &writer.DetachBuffer().map_err(fail)?,
            BitmapPixelFormat::Bgra8,
            width as i32,
            height as i32,
        )
        .map_err(fail)?;
        let result = self
            .0
            .RecognizeAsync(&bitmap)
            .map_err(fail)?
            .get()
            .map_err(fail)?;
        let lines = result.Lines().map_err(fail)?;
        let texts: Vec<String> = lines
            .into_iter()
            .filter_map(|line| line.Text().ok())
            .map(|t| t.to_string())
            .collect();
        Ok(texts.join("/"))
    }
}
