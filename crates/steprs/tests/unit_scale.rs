use steprs::analyze_step;

#[test]
fn ap214_metre_units_scales_to_mm() {
    let content = include_str!("../../../samples/ap214_medium.step");
    let result = analyze_step(content).expect("analyze");
    assert!(
        (result.length_scale - 1000.0).abs() < f64::EPSILON,
        "expected metre→mm scale, got {}",
        result.length_scale
    );
    let f = &result.features.features[0];
    let r = f.radius.expect("radius");
    assert!(r > 1.0, "radius should be mm-scale, got {r}");
}
