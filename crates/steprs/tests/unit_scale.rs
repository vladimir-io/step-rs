use steprs::{analyze_step, PipelineOptions};

#[test]
fn customer_metre_file_scales_to_mm() {
    let content = include_str!("../../../samples/customer_00167362.step");
    let result = analyze_step(content, &PipelineOptions::default()).expect("analyze");
    assert!(
        (result.length_scale - 1000.0).abs() < f64::EPSILON,
        "expected metre→mm scale, got {}",
        result.length_scale
    );
    let f = &result.features.features[0];
    let r = f.radius.expect("radius");
    assert!(r > 1.0, "radius should be mm-scale, got {r}");
}
