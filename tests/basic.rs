use image;
use recursive_bilateral::{Buffer, bilateral_filter};

const IMG: &'static [u8] = include_bytes!("../data/farmhouse.jpg");

#[test]
fn basic_test() {
    let img = image::load_from_memory(IMG).unwrap();
    let w = img.width() as usize;
    let h = img.height() as usize;

    let mut img = img.into_rgb8();

    let mut out = img.clone();
    out.fill(0);
    let mut buf = Buffer::new();
    bilateral_filter::<3>(&mut img, &mut out, w, h, 0.12, 0.09, &mut buf);

    out.save("filter.png").expect("Failed to save");
}
