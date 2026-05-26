#![feature(float_algebraic)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

pub type F = f32;

mod bilateral;
pub use bilateral::*;

mod bilateral_f32;
pub use bilateral_f32::*;

mod bilateral_u16;
pub use bilateral_u16::bilateral_filter as bilateral_filter_u16;

mod guided_upsample;
pub use guided_upsample::*;

#[derive(Default)]
pub struct Buffer {
    buf: Vec<F>,
}

impl Buffer {
    pub fn new() -> Self {
        Self { buf: vec![] }
    }
    pub fn resize<const C: usize>(&mut self, w: usize, h: usize) {
        self.buf.resize((w * h * C + w * h) * 4, 1.);
    }

    fn components_concat<const C: usize>(&mut self, w: usize, h: usize) -> [&mut [F]; 4] {
        assert_eq!(self.buf.len(), (w * h * C + w * h) * 4);
        let (lpcf, buf) = self.buf.split_at_mut(w * h * (C + 1));
        let (rpcf, buf) = buf.split_at_mut(w * h * (C + 1));
        let (dpcf, buf) = buf.split_at_mut(w * h * (C + 1));
        let (upcf, buf) = buf.split_at_mut(w * h * (C + 1));

        assert_eq!(buf, &[]);
        [lpcf, rpcf, dpcf, upcf]
    }
}

pub fn diff_factor<const C: usize>(a: [u8; C], b: [u8; C]) -> u8 {
    match C {
        1 => a[0].abs_diff(b[0]),
        3 => {
            let r = a[0].abs_diff(b[0]) >> 2;
            let g = a[1].abs_diff(b[1]) >> 1;
            let b = a[2].abs_diff(b[2]) >> 2;
            r + g + b
        }
        c => todo!("Not implemented for {c} channels"),
    }
}

pub fn diff_factor_f32<const C: usize>(a: [f32; C], b: [f32; C]) -> f32 {
    let mut sum = 0.;
    for i in 0..C {
        sum += (a[i] - b[i]).abs()
    }
    sum
}

pub fn diff_factor_u16<const C: usize>(a: [u16; C], b: [u16; C]) -> u16 {
    match C {
        1 => a[0].abs_diff(b[0]),
        3 => {
            let r = a[0].abs_diff(b[0]) >> 2;
            let g = a[1].abs_diff(b[1]) >> 1;
            let b = a[2].abs_diff(b[2]) >> 2;
            r + g + b
        }
        c => todo!("Not implemented for {c} channels"),
    }
}

#[inline]
pub(crate) fn sample_nn_half<const C: usize>(
    data: &[[u8; C]],
    w: usize,
    h: usize,

    og_x: usize,
    og_y: usize,
) -> [u8; C] {
    debug_assert!(og_x < w * 2, "{og_x} {w}");
    debug_assert!(og_y < h * 2, "{og_y} {h}");
    debug_assert_eq!(data.len(), w * h);
    let i = (og_x >> 1) + (og_y >> 1) * w;

    unsafe { *data.get_unchecked(i) }
}
