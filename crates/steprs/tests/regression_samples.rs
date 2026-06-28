use std::fs;
use std::path::PathBuf;

use steprs::{analyze_step, PipelineOptions};
use steprs_core::ManufacturingFeatureKind;

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

fn run_sample(name: &str) {
    let path = samples_dir().join(name);
    let content = fs::read_to_string(&path).expect("read sample");
    let result = analyze_step(&content, &PipelineOptions::default()).expect("analyze");
    assert!(
        result.stats.record_count > 0,
        "{name}: no records"
    );
    assert!(
        result.toolpath.as_ref().map(|t| t.segments.len()).unwrap_or(0) > 0
            || result.features.features.is_empty(),
        "{name}: expected toolpath or no features"
    );
}

#[test]
fn regression_cylinder_block() {
    run_sample("cylinder_block.step");
}

#[test]
fn regression_rectangular_pocket() {
    let content = fs::read_to_string(samples_dir().join("rectangular_pocket.step")).unwrap();
    let result = analyze_step(&content, &PipelineOptions::default()).unwrap();

    assert_eq!(result.stats.application_protocol, "AP214");

    let pockets: Vec<_> = result
        .features
        .features
        .iter()
        .filter(|f| f.kind == ManufacturingFeatureKind::RectangularPocket)
        .collect();
    assert!(
        !pockets.is_empty(),
        "expected rectangular pocket on production sample"
    );

    let pocket = pockets[0];
    let w = pocket.width.unwrap();
    let l = pocket.length.unwrap();
    assert!((w - 30.0).abs() < 2.0 || (w - 40.0).abs() < 2.0, "width {w}");
    assert!((l - 40.0).abs() < 2.0 || (l - 30.0).abs() < 2.0, "length {l}");
    assert!(pocket.depth.unwrap_or(0.0) >= 15.0);

    let tp = result.toolpath.as_ref().expect("toolpath");
    assert!(tp.stats.linear_count >= 4, "zigzag cuts expected");

    let sim = result.stock_simulation.as_ref().expect("stock sim");
    assert!(sim.removed_volume_mm3 > 100.0);
}

#[test]
fn regression_faceted_prism() {
    run_sample("faceted_prism.step");
}

#[test]
fn ap242_schema_detected() {
    let content = fs::read_to_string(samples_dir().join("faceted_prism.step")).unwrap();
    let result = analyze_step(&content, &PipelineOptions::default()).unwrap();
    assert_eq!(result.stats.application_protocol, "AP242");
    assert!(!result.mesh.vertices.is_empty() || !result.mesh.indices.is_empty());
}

#[test]
fn ap214_pocket_sample_protocol() {
    let content = fs::read_to_string(samples_dir().join("rectangular_pocket.step")).unwrap();
    let result = analyze_step(&content, &PipelineOptions::default()).unwrap();
    assert_eq!(result.stats.application_protocol, "AP214");
}
