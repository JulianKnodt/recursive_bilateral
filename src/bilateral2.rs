use super::{Buffer, F, diff_factor};

pub fn bilateral_filter<const C: usize, const C_PLUS_1: usize>(
    img_src: &[u8],
    img_dst: &mut [u8],
    w: usize,
    h: usize,
    sigma_spatial: F,
    sigma_range: F,

    buf: &mut Buffer,
) {
    assert_eq!(C+1, C_PLUS_1);
    assert_eq!(img_src.len(), w * h * C);
    assert_eq!(img_dst.len(), w * h * C);

    let alpha_f = (-std::f64::consts::SQRT_2 as F / (sigma_spatial * 255.) as F).exp();
    let inv_alpha_f = 1. - alpha_f;

    let inv_sigma_range = 1.0 / (sigma_range * 255.);
    let range_table_f = std::array::from_fn::<F, { u8::MAX as usize + 1 }, _>(|i| {
        alpha_f * (-(i as F) * inv_sigma_range).exp()
    });

    buf.resize::<C>(w, h);
    let comps = buf.components_concat::<C>(w, h);
    let [lp, rp, dp, up] = comps.map(|c| {
        let (cp, rem) = c.as_chunks_mut::<{ C_PLUS_1 }>();
        assert_eq!(rem, &[]);
        assert_eq!(cp.len(), w * h);
        cp
    });
    /*
    let [mut lp, mut rp, mut dp, mut up] = [vec![[0f32; 4]], vec![], vec![], vec![]];
    */

    // XXX do separate loops optimize better?
    for y in 0..h {
        unsafe { lp.get_unchecked_mut(y * w)[C] = 1. };
        unsafe { rp.get_unchecked_mut((y + 1) * w - 1)[C] = 1. };
    }

    let (src_color, rem) = img_src.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(src_color.len(), w * h);

    // left pass
    {
        for y in 0..h {
            let i = y * w;

            let lp_i = unsafe { lp.get_unchecked_mut(i) };
            let src_color_i = unsafe { src_color.get_unchecked(i) };
            for c in 0..C {
                lp_i[c] = src_color_i[c] as F;
            }

            debug_assert_eq!(lp_i[C], 1.);

            for curr in i + 1..i + w {
                let prev = curr - 1;

                let src_curr = unsafe { *src_color.get_unchecked(curr) };
                let src_prev = unsafe { *src_color.get_unchecked(prev) };
                let diff = diff_factor(src_curr, src_prev);
                let alpha_f = unsafe { *range_table_f.get_unchecked(diff as usize) };

                // TODO pre-add inv_alpha_f to all factors?
                /*
                unsafe {
                    *lp_f.get_unchecked_mut(curr) =
                        inv_alpha_f + alpha_f * *lp_f.get_unchecked(prev);
                }
                */

                let [lp_curr, lp_prev] = unsafe { lp.get_disjoint_unchecked_mut([curr, prev]) };
                for c in 0..C {
                    lp_curr[c] = inv_alpha_f * src_curr[c] as F + alpha_f * lp_prev[c];
                }
                lp_curr[C] = inv_alpha_f + alpha_f * lp_prev[C];
            }
        }
    }

    // right pass
    {
        for y in 0..h {
            let i = w * y;
            let last_i = i + w - 1;
            //rp_f[last_i] = 1.;
            let rp_last_i = unsafe { rp.get_unchecked_mut(last_i) };
            let src_color_last_i = unsafe { src_color.get_unchecked(last_i) };
            for c in 0..C {
                rp_last_i[c] = src_color_last_i[c] as F;
            }
            debug_assert_eq!(rp_last_i[C], 1.);

            for x in (0..w - 1).rev() {
                let curr = i + x;
                let prev = curr + 1;
                let src_curr = unsafe { *src_color.get_unchecked(curr) };
                let src_prev = unsafe { *src_color.get_unchecked(prev) };
                let diff = diff_factor(src_curr, src_prev);

                let alpha_f = unsafe { range_table_f.get_unchecked(diff as usize) };

                let [rp_curr, rp_prev] = unsafe { rp.get_disjoint_unchecked_mut([curr, prev]) };
                for c in 0..C {
                    rp_curr[c] = inv_alpha_f * src_curr[c] as F + alpha_f * rp_prev[c];
                }
                rp_curr[C] = inv_alpha_f + alpha_f * rp_prev[C];
            }
        }
    }

    // vertical pass will be applied on top on horizontal pass, while using pixel differences from original image
    // result color stored in 'm_left_pass_color' and vertical pass will use it as source color
    {
        for i in 0..w * h {
            // average color divided by average factor
            let lp_i = unsafe { lp.get_unchecked_mut(i) };
            let rp_i = unsafe { rp.get_unchecked(i) };

            // TODO there's not much perf diff switching to div vs recip?
            //let fac = unsafe { *lp_f.get_unchecked(i) + *rp_f.get_unchecked(i) };
            let fac = (lp_i[C] + rp_i[C]).recip();

            for c in 0..C {
                lp_i[c] = (lp_i[c] + rp_i[c]) * fac;
            }
        }
    }

    // down pass
    let sch = lp;
    /*
    let (sch, rem) = lp_c.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(sch.len(), w * h);
    */
    {
        //dp_f[0..w].fill(1.);
        /*
        unsafe {
            dpc
                .get_unchecked_mut(0..w)
                .copy_from_slice(&sch.get_unchecked(0..w));
        }
        */
        for x in 0..w {
            let dp_x = unsafe { dp.get_unchecked_mut(x) };
            let sch_x = unsafe { sch.get_unchecked(x) };
            for c in 0..C {
                dp_x[c] = sch_x[c];
            }
            dp_x[C] = 1.;
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
                /*
                unsafe {
                    *dp_f.get_unchecked_mut(curr) =
                        inv_alpha_f + alpha_f * *dp_f.get_unchecked(prev);
                }
                */

                let [dp_curr, dp_prev] = unsafe { dp.get_disjoint_unchecked_mut([curr, prev]) };
                let sch_curr = unsafe { sch.get_unchecked(curr) };
                for c in 0..C {
                    dp_curr[c] = inv_alpha_f * sch_curr[c] + alpha_f * dp_prev[c];
                }
                dp_curr[C] = inv_alpha_f + alpha_f * dp_prev[C];
            }
        }
    }

    /*
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
    */
}
