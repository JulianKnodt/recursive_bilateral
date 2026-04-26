#![allow(unused)]

pub mod bilateral_filter2;
pub use bilateral_filter2::{Buffer, bilateral_filter};

type F = f32;

/*
pub fn recursive_bilateral<const C: usize>(
    img: &[u8],
    output: &mut [u8],
    w: usize,
    h: usize,

    sigma_range: F,
    sigma_spatial: F,

    buffer: &mut Vec<u8>,
) {
    //assert_eq!(img.len(), w * h * C);
    //assert_eq!(output.len(), w * h * C);
    output.copy_from_slice(&img);
    buffer.resize((w * h * C + w * h + w * C + w) * 2, 0);

    let (img_out, buffer) = buffer.split_at_mut(h * w * C);
    let (img_tmp, buffer) = buffer.split_at_mut(h * w * C);

    let (map_factor_a, buffer) = buffer.split_at_mut(w * h);
    let (map_factor_b, buffer) = buffer.split_at_mut(w * h);

    let (slice_factor_a, buffer) = buffer.split_at_mut(w * C);
    let (slice_factor_b, buffer) = buffer.split_at_mut(w * C);

    let (slice_factor_b, buffer) = buffer.split_at_mut(w);
    let (slice_factor_b, buffer) = buffer.split_at_mut(w);

    assert!(buffer.is_empty());

    let mut range_table = [0.; 256];
    let inv_sigma_range = 1. / (sigma_range * 255.);
    for i in 0..=u8::MAX as usize {
        range_table[i] = (-(i as F) * inv_sigma_range).exp();
    }

    let alpha = -std::f32::consts::SQRT_2 / (sigma_spatial * w as F);
    let m1_alpha = 1. - alpha;

    for y in 0..h {
        let (mut tmp, rem) = img_tmp[y * w * C..(y + 1) * w * C].as_chunks_mut::<C>();
        assert_eq!(rem, []);
        let (mut inp, rem) = img[y * w * C..(y + 1) * w * C].as_chunks::<C>();
        assert_eq!(rem, []);
        let (mut tex, rem) = img[y * w * C..(y + 1) * w * C].as_chunks::<C>();
        assert_eq!(rem, []);

        let mut prev_y = inp[0];
        tmp[0].copy_from_slice(&inp[0]);

        let mut prev_t = tex[0];

        let tmp_factor_row = &mut map_factor_a[y * w..(y + 1) * w];
        let mut fp = 1;
        tmp_factor_row[0] = fp;

        for x in 1..w {
            let mut curr_t = tex[x];
            /*
            let d_rgb = std::array::from_fn(|i| curr_t[i].abs_diff(prev_t[i]));
            let range_dist = if c == 1 {
              d_rgb[0]
            } else {
              // TODO make this a checked add?
              (((drgb[0] << 1) + drgb[1] + drgb[2]) >> 2);
            };

            let weight = range_table[range_dist];
            let alpha_ = weight*alpha;

            let tn = tmp.next();
            let curr =
            for i in 0..c {
              tn[i] =
            *temp_x++ = ycr = inv_alpha_*(*in_x++) + alpha_*ypr;
            *temp_x++ = ycg = inv_alpha_*(*in_x++) + alpha_*ypg;
            *temp_x++ = ycb = inv_alpha_*(*in_x++) + alpha_*ypb;
            }
            */
        }
    }
}
*/
