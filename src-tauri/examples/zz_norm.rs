// Temporary: a screenshot as Aoi reads it (`screen::normalize`), saved.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let image = image::open(&a[0]).unwrap().to_rgb8();
    aoi::screen::normalize(&image).save(&a[1]).unwrap();
}
