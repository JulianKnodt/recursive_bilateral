pub type F = f32;

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
        _ => todo!(),
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
    // XXX do separate loops optimize better?
    for y in 0..h {
        unsafe { *lp_f.get_unchecked_mut(y * w) = 1. };
        unsafe { *rp_f.get_unchecked_mut((y + 1) * w - 1) = 1. };
    }

    let (src_color, rem) = img_src.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(src_color.len(), w * h);

    // left pass
    {
        let (lpc, rem) = lp_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(lpc.len(), w * h);

        for y in 0..h {
            let i = y * w;

            let lpc_i = unsafe { lpc.get_unchecked_mut(i) };
            let src_color_i = unsafe { src_color.get_unchecked(i) };
            for c in 0..C {
                lpc_i[c] = src_color_i[c] as F;
            }

            debug_assert_eq!(lp_f[i], 1.);

            for curr in i + 1..i + w {
                let prev = curr - 1;

                let src_curr = unsafe { *src_color.get_unchecked(curr) };
                let src_prev = unsafe { *src_color.get_unchecked(prev) };
                let diff = diff_factor(src_curr, src_prev);
                let alpha_f = unsafe { *range_table_f.get_unchecked(diff as usize) };

                // TODO pre-add inv_alpha_f to all factors?
                unsafe {
                    *lp_f.get_unchecked_mut(curr) =
                        inv_alpha_f + alpha_f * *lp_f.get_unchecked(prev);
                }

                let [lpc_curr, lpc_prev] = unsafe { lpc.get_disjoint_unchecked_mut([curr, prev]) };
                for c in 0..C {
                    lpc_curr[c] = inv_alpha_f * src_curr[c] as F + alpha_f * lpc_prev[c];
                }
            }
        }
    }

    // right pass
    {
        let (rpc, rem) = rp_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(rpc.len(), w * h);

        for y in 0..h {
            let i = w * y;
            let last_i = i + w - 1;
            debug_assert_eq!(rp_f[last_i], 1.);
            //rp_f[last_i] = 1.;
            let rpc_last_i = unsafe { rpc.get_unchecked_mut(last_i) };
            let src_color_last_i = unsafe { src_color.get_unchecked(last_i) };
            for c in 0..C {
                rpc_last_i[c] = src_color_last_i[c] as F;
            }

            for x in (0..w - 1).rev() {
                let curr = i + x;
                let prev = curr + 1;
                let src_curr = unsafe { *src_color.get_unchecked(curr) };
                let src_prev = unsafe { *src_color.get_unchecked(prev) };
                let diff = diff_factor(src_curr, src_prev);

                let alpha_f = unsafe { range_table_f.get_unchecked(diff as usize) };
                unsafe {
                    *rp_f.get_unchecked_mut(curr) =
                        inv_alpha_f + alpha_f * *rp_f.get_unchecked(prev);
                }

                let [rpc_curr, rpc_prev] = unsafe { rpc.get_disjoint_unchecked_mut([curr, prev]) };
                for c in 0..C {
                    rpc_curr[c] = inv_alpha_f * src_curr[c] as F + alpha_f * rpc_prev[c];
                }
            }
        }
    }

    // vertical pass will be applied on top on horizontal pass, while using pixel differences from original image
    // result color stored in 'm_left_pass_color' and vertical pass will use it as source color
    {
        let (lc, rem) = lp_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(lc.len(), w * h);
        let (rc, rem) = rp_c.as_chunks::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(rc.len(), w * h);

        for i in 0..w * h {
            // average color divided by average factor

            // TODO there's not much perf diff switching to div vs recip?
            let fac = unsafe { *lp_f.get_unchecked(i) + *rp_f.get_unchecked(i) };
            let fac = fac.recip();

            let lc_i = unsafe { lc.get_unchecked_mut(i) };
            let rc_i = unsafe { rc.get_unchecked(i) };
            for c in 0..C {
                lc_i[c] = (lc_i[c] + rc_i[c]) * fac;
            }
        }
    }

    // down pass
    let (sch, rem) = lp_c.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(sch.len(), w * h);
    {
        let (dpc, rem) = dp_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(dpc.len(), w * h);

        dp_f[0..w].fill(1.);
        /*
        unsafe {
            dpc
                .get_unchecked_mut(0..w)
                .copy_from_slice(&sch.get_unchecked(0..w));
        }
        */
        for x in 0..w {
            dpc[x] = sch[x];
        }

        /* NOTE technically could be 1..h but for reduced computation later */
        for y in 0..h - 1 {
            for x in 0..w {
                let prev = x + y * w;
                let curr = prev + w;
                let src_color_curr = unsafe { *src_color.get_unchecked(curr) };
                let src_color_prev = unsafe { *src_color.get_unchecked(prev) };
                let diff = diff_factor(src_color_curr, src_color_prev);

                let alpha_f = unsafe { *range_table_f.get_unchecked(diff as usize) };
                unsafe {
                    *dp_f.get_unchecked_mut(curr) =
                        inv_alpha_f + alpha_f * *dp_f.get_unchecked(prev);
                }

                let [dpc_curr, dpc_prev] = unsafe { dpc.get_disjoint_unchecked_mut([curr, prev]) };
                let sch_curr = unsafe { sch.get_unchecked(curr) };
                for c in 0..C {
                    dpc_curr[c] = inv_alpha_f * sch_curr[c] + alpha_f * dpc_prev[c];
                }
            }
        }
    }

    // up pass
    {
        let (upc, rem) = up_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(upc.len(), w * h);

        up_f[w*(h-1)..w*h].fill(1.);
        /*
        let r = w*(h-1)..w*h;
        upc[r.clone()].copy_from_slice(&sch[r]);
        */
        for x in 0..w {
            let i = w * (h - 1) + x;
            upc[i] = sch[i];
        }

        for y in (0..h - 1).rev() {
            for x in 0..w {
                let curr = x + y * w;
                let prev = curr + w;
                let src_color_curr = unsafe { *src_color.get_unchecked(curr) };
                let src_color_prev = unsafe { *src_color.get_unchecked(prev) };
                let diff = diff_factor(src_color_curr, src_color_prev);

                let alpha_f = unsafe { range_table_f.get_unchecked(diff as usize) };

                unsafe {
                    *up_f.get_unchecked_mut(curr) =
                        inv_alpha_f + alpha_f * up_f.get_unchecked(prev);
                }

                let [upc_curr, upc_prev] = unsafe { upc.get_disjoint_unchecked_mut([curr, prev]) };
                let sch_curr = unsafe { sch.get_unchecked(curr) };
                for c in 0..C {
                    upc_curr[c] = inv_alpha_f * sch_curr[c] + alpha_f * upc_prev[c];
                }
            }
        }
    }

    /*
     */
    let (dst, rem) = img_dst.as_chunks_mut::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(dst.len(), w * h);
    let (uc, rem) = up_c.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(uc.len(), w * h);
    let (dc, rem) = dp_c.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(dc.len(), w * h);

    for i in 0..w * h {
        // average color divided by average factor
        let fac = unsafe { *up_f.get_unchecked(i) + *dp_f.get_unchecked(i) };
        let fac = fac.recip();

        let dst_i = unsafe { dst.get_unchecked_mut(i) };
        let uc_i = unsafe { uc.get_unchecked(i) };
        let dc_i = unsafe { dc.get_unchecked(i) };
        for c in 0..C {
            dst_i[c] = ((uc_i[c] + dc_i[c]) * fac).clamp(0., 255.) as u8;
        }
    }
}
