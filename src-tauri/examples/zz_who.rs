// Temporary: each screenshot in a folder: the student it names, "plate" where a student screen names none, else "-".
fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let r = aoi::Recognizer::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data"))
        .unwrap();
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .collect();
    files.sort();
    for f in files {
        let image = image::open(&f).unwrap().to_rgb8();
        let image = aoi::screen::normalize(&image).into_owned();
        let who = match r.read_student(&image) {
            Some((Some(id), _)) => id.to_string(),
            Some((None, _)) => "plate".into(),
            None => "-".into(),
        };
        println!(
            "{}\t{}x{}\t{who}",
            f.file_name().unwrap().to_string_lossy(),
            image.width(),
            image.height()
        );
    }
}
