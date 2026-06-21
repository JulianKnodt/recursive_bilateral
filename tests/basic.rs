use image;
use recursive_bilateral::{Buffer, bilateral_filter, guided_2x};

const IMG: &'static [u8] = include_bytes!("../data/farmhouse.jpg");
const CKBD: &'static [u8] = include_bytes!("../data/checkerboard.png");

#[test]
fn basic_test() {
    let img = image::load_from_memory(IMG).unwrap();
    let w = img.width() as usize;
    let h = img.height() as usize;

    let mut img = img.into_rgb8();

    let mut out = img.clone();
    out.fill(0);
    let mut buf = Buffer::new();
    bilateral_filter::<3, 4>(&mut img, &mut out, w, h, 0.12, 0.09, &mut buf);

    out.save("filter2.png").expect("Failed to save");
}

#[test]
fn guided_upsample_test() {
    let img = image::load_from_memory(IMG).unwrap();
    let w = img.width() as usize;
    let h = img.height() as usize;

    let ckbd = image::load_from_memory(CKBD).unwrap().into_rgb8();

    let img = img.into_rgb8();

    let filter = image::imageops::FilterType::Nearest;
    let low_res = image::imageops::resize(&img, w as u32 / 2, h as u32 / 2, filter);
    let guide = image::imageops::resize(&ckbd, w as u32, h as u32, filter);

    let mut out = img.clone();
    out.fill(0);
    let mut buf = Buffer::new();
    guided_2x::<3, 4>(&low_res, &mut out, &guide, w, h, 0.12, 0.09, &mut buf);

    out.save("upsample_2x.png").expect("Failed to save");
}
