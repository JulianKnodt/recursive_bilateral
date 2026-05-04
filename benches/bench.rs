#![feature(test)]

extern crate test;

use recursive_bilateral::{Buffer, bilateral_filter};

use image;

const IMG: &'static [u8] = include_bytes!("../data/farmhouse.jpg");

#[bench]
fn bench_rb(b: &mut test::Bencher) {
    let img = image::load_from_memory(IMG).unwrap();
    let w = img.width() as usize;
    let h = img.height() as usize;

    let img = img.into_rgb8();
    let mut out = img.clone();
    let mut buf = Buffer::default();
    buf.resize::<3>(w, h);

    b.iter(|| bilateral_filter::<3>(&img, &mut out, w, h, 0.12, 0.09, &mut buf));
}
