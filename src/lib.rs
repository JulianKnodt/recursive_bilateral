pub type F = f32;

#[derive(Default)]
pub struct Buffer {
    buf: Vec<F>,
}

impl Buffer {
    pub fn new() -> Self {
        Self { buf: vec![] }
    }
    fn resize<const C: usize>(&mut self, w: usize, h: usize) {
        self.buf.resize((w * h * C + w * h) * 4, 1.);
    }
    fn components<const C: usize>(&mut self, w: usize, h: usize) -> ([&mut [F]; 4], [&mut [F]; 4]) {
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
    let comp_diff: [u8; C] = std::array::from_fn(|i| a[i].abs_diff(b[i]));

    match comp_diff.as_slice() {
        &[] => panic!(),
        &[v] => v,
        // average below
        &[_, _] => panic!(),
        &[r, g, b] => (r >> 2) + (g >> 1) + (b >> 2),
        //&[r, g, b] => ((r + b) >> 2) + (g >> 1),
        &[_r, _g, _b, _a] => todo!(),
        x => panic!("{x:?}"),
    }
}

pub fn bilateral_filter<const C: usize>(
    img_src: &[u8],
    img_dst: &mut [u8],
    w: usize,
    h: usize,
    sigma_spatial: F,
    sigma_range: F,

    buf: &mut Buffer,
) {
    assert_eq!(img_src.len(), w * h * C);
    assert_eq!(img_dst.len(), w * h * C);

    let alpha_f = (-std::f64::consts::SQRT_2 as F / (sigma_spatial * 255.) as F).exp();
    let inv_alpha_f = 1. - alpha_f;

    let inv_sigma_range = 1.0 / (sigma_range * 255.);
    let range_table_f = std::array::from_fn::<F, { u8::MAX as usize + 1 }, _>(|i| {
        alpha_f * (-(i as F) * inv_sigma_range).exp()
    });

    buf.resize::<C>(w, h);
    let ([lp_c, rp_c, dp_c, up_c], [lp_f, rp_f, dp_f, up_f]) = buf.components::<C>(w, h);
    for y in 0..h {
      unsafe { *lp_f.get_unchecked_mut(y * w) = 1. };
      unsafe { *rp_f.get_unchecked_mut((y+1) * w - 1) = 1. };
    }

    let (src_color, rem) = img_src.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(src_color.len(), w * h);


    // left pass
    {
        /*
        let (l_p_color, rem) = lp_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(l_p_color.len(), w * h);
        */

        for y in 0..h {
            let idx = y * w;

            for c in 0..C {
                unsafe {
                    *lp_c.get_unchecked_mut(idx * C + c) = *lp_c.get_unchecked(idx * C + c) as F;
                }
            }

            debug_assert_eq!(lp_f[idx], 1.);

            for x in 1..w {
                let curr = idx + x;
                let prev = curr - 1;

                /*
                let diff = unsafe {
                    let sc: [_; C] = std::array::from_fn(|c| *img_src.get_unchecked(curr * C + c));
                    let sp: [_; C] = std::array::from_fn(|c| *img_src.get_unchecked(prev * C + c));
                    diff_factor(sc, sp)
                };
                */
                let diff = unsafe {
                    diff_factor(
                        *src_color.get_unchecked(curr),
                        *src_color.get_unchecked(prev),
                    )
                };
                let alpha_f = unsafe { *range_table_f.get_unchecked(diff as usize) };

                unsafe {
                    *lp_f.get_unchecked_mut(curr) =
                        inv_alpha_f + alpha_f * *lp_f.get_unchecked(prev);
                }

                for c in 0..C {
                    unsafe {
                        *lp_c.get_unchecked_mut(curr * C + c) = inv_alpha_f
                            * *img_src.get_unchecked(curr * C + c) as F
                            + alpha_f * *lp_c.get_unchecked(prev * C + c);
                    }
                }
            }
        }
    }

    // right pass
    {
        /*
        let (r_p_color, rem) = rp_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(src_color.len(), w * h);
         */

        for y in 0..h {
            let i = w * y;
            let last_i = i + w - 1;
            debug_assert_eq!(rp_f[last_i], 1.);
            //rp_f[last_i] = 1.;
            unsafe {
                for c in 0..C {
                    //.map(|v| v as F);
                    *rp_c.get_unchecked_mut(last_i * C + c) =
                        *img_src.get_unchecked(last_i * C + c) as F;
                }
            }
            for x in (0..w - 1).rev() {
                let curr = i + x;
                let prev = curr + 1;
                let diff = diff_factor(src_color[curr], src_color[prev]);

                let alpha_f = unsafe { range_table_f.get_unchecked(diff as usize) };
                unsafe {
                    //rp_f[c] = inv_alpha_f + alpha_f * rp_f[p];
                    *rp_f.get_unchecked_mut(curr) =
                        inv_alpha_f + alpha_f * *rp_f.get_unchecked(prev);
                }

                for c in 0..C {
                    rp_c[curr * C + c] =
                        inv_alpha_f * img_src[curr * C + c] as F + alpha_f * rp_c[prev * C + c];
                }

                /*
                r_p_color[curr] = std::array::from_fn(|i| {
                    inv_alpha_f * src_color[curr][i] as F + alpha_f * r_p_color[prev][i]
                });
                */
            }
        }
    }

    // vertical pass will be applied on top on horizontal pass, while using pixel differences from original image
    // result color stored in 'm_left_pass_color' and vertical pass will use it as source color
    {
        /*
        let (lc, rem) = lp_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);
        let (rc, rem) = rp_c.as_chunks::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(lc.len(), w * h);
        assert_eq!(rc.len(), w * h);
        */

        for i in 0..w * h {
            // average color divided by average factor
            let fac = (lp_f[i] + rp_f[i]).recip();

            for c in 0..C {
                /*
                let avg = lc[i][c] + rc[i][c];
                *unsafe { lc[i].get_unchecked_mut(c) } = fac * avg;
                */
                let avg = lp_c[i * C + c] + rp_c[i * C + c];
                lp_c[i * C + c] = fac * avg;
            }
        }
    }

    if true {
      return;
    }

    // down pass
    let (sch, rem) = lp_c.as_chunks::<C>();
    assert_eq!(rem, &[]);
    {
        let (d_p_color, rem) = dp_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);

        for x in 0..w {
            dp_f[x] = 1.;
            d_p_color[x] = sch[x];
        }

        for y in 1..h {
            for x in 0..w {
                let c = x + y * w;
                let p = x + (y - 1) * w;
                let diff = diff_factor(src_color[c], src_color[p]);

                let alpha_f = range_table_f[diff as usize];
                dp_f[c] = inv_alpha_f + alpha_f * dp_f[p];
                d_p_color[c] = std::array::from_fn(|i| {
                    inv_alpha_f * sch[c][i] as F + alpha_f * d_p_color[p][i]
                });
            }
        }
    }

    // up pass
    {
        let (u_p_color, rem) = up_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);

        for x in 0..w {
            let i = w * (h - 1) + x;
            up_f[i] = 1.;
            u_p_color[i] = sch[i];
        }

        for y in (0..h - 1).rev() {
            for x in 0..w {
                let c = x + y * w;
                let p = c + w;
                let diff = diff_factor(src_color[c], src_color[p]);

                let alpha_f = range_table_f[diff as usize];
                up_f[c] = inv_alpha_f + alpha_f * up_f[p];
                u_p_color[c] = std::array::from_fn(|i| {
                    inv_alpha_f * sch[c][i] as F + alpha_f * u_p_color[p][i]
                });
            }
        }
    }

    let (dst, rem) = img_dst.as_chunks_mut::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(dst.len(), w * h);
    {
        let (uc, rem) = up_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(uc.len(), w * h);
        let (dc, rem) = dp_c.as_chunks::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(dc.len(), w * h);

        for i in 0..w * h {
            // average color divided by average factor
            let fac = (up_f[i] + dp_f[i]).recip();

            for c in 0..C {
                let avg = uc[i][c] + dc[i][c];
                //assert!((fac * avg) < 255., "{}", fac * avg);
                dst[i][c] = (fac * avg).clamp(0., 255.) as u8;
            }
        }
    }
}
