use recursive_bilateral::bilateral_filter;

fn main() {
    const C: usize = 1;
    bilateral_filter::<C>(
        &[0; 100 * 100 * C],
        &mut [0; 100 * 100 * C],
        100,
        100,
        1.,
        1.,
        &mut Default::default(),
    );
}
