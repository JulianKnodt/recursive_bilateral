use super::{Buffer, F, diff_factor, sample_nn_half};

pub fn guided_2x<const C: usize>(
    // half resolution src to upsample
    img_src: &[u8],
    dst_w: usize,
    dst_h: usize,

    // destination location
    img_dst: &mut [u8],
    // image to use as guide when upsampling
    guide_img: &[u8],

    sigma_spatial: F,
    sigma_range: F,

    buf: &mut Buffer,
) {
    assert_eq!(img_src.len(), dst_w * dst_h * C / 4);
    assert_eq!(dst_w % 2, 0);
    assert_eq!(dst_h % 2, 0);

    assert_eq!(img_dst.len(), dst_w * dst_h * C);
    assert_eq!(guide_img.len(), dst_w * dst_h * C);

    let alpha_f = (-std::f64::consts::SQRT_2 as F / (sigma_spatial * 255.) as F).exp();
    let inv_alpha_f = 1. - alpha_f;

    let inv_sigma_range = 1.0 / (sigma_range * 255.);
    let range_table_f = std::array::from_fn::<F, { u8::MAX as usize + 1 }, _>(|i| {
        alpha_f * (-(i as F) * inv_sigma_range).exp()
    });

    let w = dst_w;
    let h = dst_h;

    let src_w = w / 2;
    let src_h = h / 2;

    buf.resize::<C>(w, h);
    let ([lp_c, rp_c, dp_c, up_c], [lp_f, rp_f, dp_f, up_f]) = buf.components::<C>(w, h);
    // XXX do separate loops optimize better?
    for y in 0..h {
        unsafe { *lp_f.get_unchecked_mut(y * w) = 1. };
        unsafe { *rp_f.get_unchecked_mut((y + 1) * w - 1) = 1. };
    }

    let (src_color, rem) = img_src.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(src_color.len(), src_w * src_h);

    let (guide_color, rem) = guide_img.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(guide_color.len(), w * h);

    // left pass
    {
        let (lpc, rem) = lp_c.as_chunks_mut::<C>();
        assert_eq!(rem, &[]);
        assert_eq!(lpc.len(), w * h);

        for y in 0..h {
            let i = y * w;

            let lpc_i = unsafe { lpc.get_unchecked_mut(i) };
            let src_color_i = sample_nn_half(src_color, src_w, src_h, 0, y);
            for c in 0..C {
                lpc_i[c] = src_color_i[c] as F;
            }

            debug_assert_eq!(lp_f[i], 1.);

            for x in 1..w {
                let curr = i + x;
                let prev = curr - 1;

                let guide_curr = unsafe { *guide_color.get_unchecked(curr) };
                let guide_prev = unsafe { *guide_color.get_unchecked(prev) };
                let diff = diff_factor(guide_curr, guide_prev);
                let alpha_f = unsafe { *range_table_f.get_unchecked(diff as usize) };

                // TODO pre-add inv_alpha_f to all factors?
                unsafe {
                    *lp_f.get_unchecked_mut(curr) =
                        inv_alpha_f + alpha_f * *lp_f.get_unchecked(prev);
                }

                let [lpc_curr, lpc_prev] = unsafe { lpc.get_disjoint_unchecked_mut([curr, prev]) };
                let src_curr = sample_nn_half(src_color, src_w, src_h, x, y);
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
            //let src_color_last_i = unsafe { src_color.get_unchecked(last_i) };
            let src_color_last_i = sample_nn_half(src_color, src_w, src_h, w - 1, y);
            for c in 0..C {
                rpc_last_i[c] = src_color_last_i[c] as F;
            }

            for x in (0..w - 1).rev() {
                let curr = i + x;
                let prev = curr + 1;
                let guide_curr = unsafe { *guide_color.get_unchecked(curr) };
                let guide_prev = unsafe { *guide_color.get_unchecked(prev) };
                let diff = diff_factor(guide_curr, guide_prev);

                let alpha_f = unsafe { range_table_f.get_unchecked(diff as usize) };
                unsafe {
                    *rp_f.get_unchecked_mut(curr) =
                        inv_alpha_f + alpha_f * *rp_f.get_unchecked(prev);
                }

                let [rpc_curr, rpc_prev] = unsafe { rpc.get_disjoint_unchecked_mut([curr, prev]) };
                let src_curr = sample_nn_half(src_color, src_w, src_h, x, y);
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
                let guide_curr = unsafe { *guide_color.get_unchecked(curr) };
                let guide_prev = unsafe { *guide_color.get_unchecked(prev) };
                let diff = diff_factor(guide_curr, guide_prev);

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

        up_f[w * (h - 1)..w * h].fill(1.);
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
                let guide_curr = unsafe { *guide_color.get_unchecked(curr) };
                let guide_prev = unsafe { *guide_color.get_unchecked(prev) };
                let diff = diff_factor(guide_curr, guide_prev);

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
