pub type F = f32;

mod bilateral;
pub use bilateral::*;

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
    // TODO change these components to instead be C+1 for each color+factor,
    // so they can be stored together
    fn components<const C: usize>(&mut self, w: usize, h: usize) -> ([&mut [F]; 4], [&mut [F]; 4]) {
        assert_eq!(self.buf.len(), (w * h * C + w * h) * 4);
        let (left_pass_color, buf) = self.buf.split_at_mut(w * h * C);
        let (right_pass_color, buf) = buf.split_at_mut(w * h * C);
        let (down_pass_color, buf) = buf.split_at_mut(w * h * C);
        let (up_pass_color, buf) = buf.split_at_mut(w * h * C);

        let (left_pass_factor, buf) = buf.split_at_mut(w * h);
        let (right_pass_factor, buf) = buf.split_at_mut(w * h);
        let (down_pass_factor, buf) = buf.split_at_mut(w * h);
        let (up_pass_factor, buf) = buf.split_at_mut(w * h);
        assert_eq!(buf, &[]);
        let colors = [
            left_pass_color,
            right_pass_color,
            down_pass_color,
            up_pass_color,
        ];
        let factors = [
            left_pass_factor,
            right_pass_factor,
            down_pass_factor,
            up_pass_factor,
        ];
        (colors, factors)
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
