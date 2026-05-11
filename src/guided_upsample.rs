use super::{Buffer, F, diff_factor, sample_nn_half};

pub fn guided_2x<const C: usize, const C_PLUS_1: usize>(
    img_src: &[u8],
    img_dst: &mut [u8],
    img_guide: &[u8],

    dst_w: usize,
    dst_h: usize,

    sigma_spatial: F,
    sigma_range: F,

    buf: &mut Buffer,
) {
    assert_eq!(
        C + 1,
        C_PLUS_1,
        "Temporary restriction due to const generics"
    );

    assert_eq!(img_src.len(), dst_w * dst_h * C / 4);
    assert_eq!(dst_w % 2, 0);
    assert_eq!(dst_h % 2, 0);
    let w = dst_w;
    let h = dst_h;

    assert_eq!(img_guide.len(), w * h * C);
    assert_eq!(img_dst.len(), w * h * C);

    let src_w = w / 2;
    let src_h = h / 2;

    let alpha_f = (-std::f64::consts::SQRT_2 as F / (sigma_spatial * 255.) as F).exp();
    let inv_alpha_f = 1. - alpha_f;

    let inv_sigma_range = 1.0 / (sigma_range * 255.);
    let range_table_f = std::array::from_fn::<F, { u8::MAX as usize + 1 }, _>(|i| {
        alpha_f * (-(i as F) * inv_sigma_range).exp()
    });

    buf.resize::<C>(w, h);
    let comps = buf.components_concat::<C>(w, h);
    let [lp, rp, dp, up] = comps.map(|c| {
        let (cp, rem) = c.as_chunks_mut::<C_PLUS_1>();
        assert_eq!(rem, &[]);
        assert_eq!(cp.len(), w * h);
        cp
    });

    // XXX do separate loops optimize better?
    for y in 0..h {
        unsafe { lp.get_unchecked_mut(y * w)[C] = 1. };
        unsafe { rp.get_unchecked_mut((y + 1) * w - 1)[C] = 1. };
    }

    let (src_color, rem) = img_src.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(src_color.len(), src_w * src_h);

    let (guide_color, rem) = img_guide.as_chunks::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(guide_color.len(), w * h);

    // left pass
    {
        for y in 0..h {
            let i = y * w;

            let lp_i = unsafe { lp.get_unchecked_mut(i) };
            let src_color_i = sample_nn_half(src_color, src_w, src_h, 0, y);
            for c in 0..C {
                lp_i[c] = src_color_i[c] as F;
            }

            debug_assert_eq!(lp_i[C], 1.);

            for x in 1..w {
                let curr = i + x;
                let prev = curr - 1;

                let guide_curr = unsafe { *guide_color.get_unchecked(curr) };
                let guide_prev = unsafe { *guide_color.get_unchecked(prev) };
                let diff = diff_factor(guide_curr, guide_prev);
                let alpha_f = unsafe { *range_table_f.get_unchecked(diff as usize) };

                let [lp_curr, lp_prev] = unsafe { lp.get_disjoint_unchecked_mut([curr, prev]) };
                let src_curr = sample_nn_half(src_color, src_w, src_h, x, y);
                for c in 0..C {
                    lp_curr[c] = inv_alpha_f
                        .algebraic_mul(src_curr[c] as F)
                        .algebraic_add(alpha_f.algebraic_mul(lp_prev[c]));
                }
                lp_curr[C] = inv_alpha_f.algebraic_add(alpha_f.algebraic_mul(lp_prev[C]));
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
            let src_color_last_i = sample_nn_half(src_color, src_w, src_h, w - 1, y);
            for c in 0..C {
                rp_last_i[c] = src_color_last_i[c] as F;
            }
            debug_assert_eq!(rp_last_i[C], 1.);

            for x in (0..w - 1).rev() {
                let curr = i + x;
                let prev = curr + 1;
                let guide_curr = unsafe { *guide_color.get_unchecked(curr) };
                let guide_prev = unsafe { *guide_color.get_unchecked(prev) };
                let diff = diff_factor(guide_curr, guide_prev);

                let alpha_f = unsafe { range_table_f.get_unchecked(diff as usize) };

                let [rp_curr, rp_prev] = unsafe { rp.get_disjoint_unchecked_mut([curr, prev]) };
                let src_curr = sample_nn_half(src_color, src_w, src_h, x, y);
                for c in 0..C {
                    rp_curr[c] = inv_alpha_f
                        .algebraic_mul(src_curr[c] as F)
                        .algebraic_add(alpha_f.algebraic_mul(rp_prev[c]));
                }
                rp_curr[C] = inv_alpha_f.algebraic_add(alpha_f.algebraic_mul(rp_prev[C]));
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
            let fac = (lp_i[C] + rp_i[C]).recip();
            for c in 0..C {
                lp_i[c] = (lp_i[c] + rp_i[c]) * fac;
            }
            /*
            let fac = lp_i[C].algebraic_add(rp_i[C]);
            for c in 0..C {
                lp_i[c] = lp_i[c].algebraic_add(rp_i[c]).algebraic_div(fac);
            }
            */
        }
    }

    // down pass
    let sch = lp;
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
                let guide_color_curr = unsafe { *guide_color.get_unchecked(curr) };
                let guide_color_prev = unsafe { *guide_color.get_unchecked(prev) };
                let diff = diff_factor(guide_color_curr, guide_color_prev);

                let alpha_f = unsafe { *range_table_f.get_unchecked(diff as usize) };

                let [dp_curr, dp_prev] = unsafe { dp.get_disjoint_unchecked_mut([curr, prev]) };
                let sch_curr = unsafe { sch.get_unchecked(curr) };
                /*
                for c in 0..C {
                    dp_curr[c] = inv_alpha_f * sch_curr[c] + alpha_f * dp_prev[c];
                }
                dp_curr[C] = inv_alpha_f + alpha_f * dp_prev[C];
                */
                for c in 0..C {
                    dp_curr[c] = inv_alpha_f
                        .algebraic_mul(sch_curr[c])
                        .algebraic_add(alpha_f.algebraic_mul(dp_prev[c]));
                }
                dp_curr[C] = inv_alpha_f.algebraic_add(alpha_f.algebraic_mul(dp_prev[C]));
            }
        }
    }

    // up pass
    {
        //up_f[w * (h - 1)..w * h].fill(1.);
        /*
        let r = w*(h-1)..w*h;
        upc[r.clone()].copy_from_slice(&sch[r]);
        */
        for x in 0..w {
            let i = w * (h - 1) + x;
            let up_i = unsafe { up.get_unchecked_mut(i) };
            let sch_i = unsafe { sch.get_unchecked(i) };
            for c in 0..C {
                up_i[c] = sch_i[c];
            }
            up_i[C] = 1.;
        }

        for y in (0..h - 1).rev() {
            for x in 0..w {
                let curr = x + y * w;
                let prev = curr + w;
                let guide_color_curr = unsafe { *guide_color.get_unchecked(curr) };
                let guide_color_prev = unsafe { *guide_color.get_unchecked(prev) };
                let diff = diff_factor(guide_color_curr, guide_color_prev);

                let alpha_f = unsafe { range_table_f.get_unchecked(diff as usize) };

                let [up_curr, up_prev] = unsafe { up.get_disjoint_unchecked_mut([curr, prev]) };
                let sch_curr = unsafe { sch.get_unchecked(curr) };
                /*
                for c in 0..C {
                    up_curr[c] = inv_alpha_f * sch_curr[c] + alpha_f * up_prev[c];
                }
                up_curr[C] = inv_alpha_f + alpha_f * up_prev[C];
                */
                for c in 0..C {
                    up_curr[c] = inv_alpha_f
                        .algebraic_mul(sch_curr[c])
                        .algebraic_add(alpha_f.algebraic_mul(up_prev[c]));
                }
                up_curr[C] = inv_alpha_f.algebraic_add(alpha_f.algebraic_mul(up_prev[C]));
            }
        }
    }

    let (dst, rem) = img_dst.as_chunks_mut::<C>();
    assert_eq!(rem, &[]);
    assert_eq!(dst.len(), w * h);

    for i in 0..w * h {
        let up_i = unsafe { up.get_unchecked(i) };
        let dp_i = unsafe { dp.get_unchecked(i) };
        let dst_i = unsafe { dst.get_unchecked_mut(i) };
        // average color divided by average factor
        let fac = (up_i[C] + dp_i[C]).recip();

        for c in 0..C {
            dst_i[c] = ((up_i[c] + dp_i[c]) * fac).clamp(0., 255.) as u8;
        }
    }
}
