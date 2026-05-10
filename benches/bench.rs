#![feature(test)]

extern crate test;

use recursive_bilateral::{Buffer, bilateral_filter, guided_2x};

use image;

const IMG: &'static [u8] = include_bytes!("../data/farmhouse.jpg");
const CKBD: &'static [u8] = include_bytes!("../data/checkerboard.png");

#[bench]
fn bench_rb(b: &mut test::Bencher) {
    let img = image::load_from_memory(IMG).unwrap();
    let w = img.width() as usize;
    let h = img.height() as usize;

    let img = img.into_rgb8();
    let mut out = img.clone();
    let mut buf = Buffer::default();
    buf.resize::<3>(w, h);

    b.iter(|| bilateral_filter::<3, 4>(&img, &mut out, w, h, 0.12, 0.09, &mut buf));
}

#[bench]
fn bench_guided_upsample(b: &mut test::Bencher) {
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
    b.iter(|| guided_2x::<3>(&low_res, w, h, &mut out, &guide, 0.12, 0.09, &mut buf));

    out.save("upsample_2x.png").expect("Failed to save");
}

#[bench]
fn bench_guided_upsample_one_channel(b: &mut test::Bencher) {
    let img = image::load_from_memory(IMG).unwrap();
    let w = img.width() as usize;
    let h = img.height() as usize;

    let ckbd = image::load_from_memory(CKBD).unwrap().into_luma8();

    let img = img.into_luma8();

    let filter = image::imageops::FilterType::Nearest;
    let low_res = image::imageops::resize(&img, w as u32 / 2, h as u32 / 2, filter);
    let guide = image::imageops::resize(&ckbd, w as u32, h as u32, filter);

    let mut out = img.clone();
    out.fill(0);
    let mut buf = Buffer::new();
    b.iter(|| guided_2x::<1>(&low_res, w, h, &mut out, &guide, 0.12, 0.09, &mut buf));

    out.save("upsample_2x_1c.png").expect("Failed to save");
}
