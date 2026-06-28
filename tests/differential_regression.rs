//! Differential regression — ground-truth STEP dimensions vs `detect_coaxial_holes`.
//!
//! Ingests fixture STEP files, runs the coaxial detection pipeline (WASM client
//! entry: `analyze_web_json`), and asserts known mathematical dimensions for
//! `cylinder_block.step`. Float compares use explicit epsilon: `(a - b).abs() < EPS`.

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use steprs::analyze_web_json;
use steprs_core::{parse_step_file, ManufacturingFeatureKind};
use steprs_features::detect_coaxial_holes;
use steprs_schema::{SchemaCache, Vec3};
use steprs_topology::build_brep;

/// Platform-stable float tolerance for dimensional regression.
const EPS: f64 = 1e-6;

/// WASM client budget for standard fixture parts (ms).
const WASM_CLIENT_BUDGET_MS: f64 = 50.0;

const FIXTURES: &[&str] = &[
    "cylinder_block.step",
    "rectangular_pocket.step",
    "faceted_prism.step",
    "customer_00167362.step",
];

/// Ground truth derived from `samples/cylinder_block.step` entity graph.
struct CylinderBlockTruth {
    outer_radius_mm: f64,
    inner_radius_mm: f64,
    outer_diameter_mm: f64,
    inner_diameter_mm: f64,
    z_step_mm: f64,
    total_depth_mm: f64,
    segment_depths_mm: [f64; 2],
    axis_origin: Vec3,
    axis_direction: Vec3,
    face_ids: [u32; 2],
    confidence: f32,
}

impl CylinderBlockTruth {
    fn new() -> Self {
        Self {
            outer_radius_mm: 12.7,
            inner_radius_mm: 6.35,
            outer_diameter_mm: 25.4,
            inner_diameter_mm: 12.7,
            z_step_mm: 10.0,
            total_depth_mm: 22.7,
            segment_depths_mm: [10.0, 12.7],
            axis_origin: Vec3::new(0.0, 0.0, 0.0),
            axis_direction: Vec3::new(0.0, 0.0, 1.0),
            face_ids: [100, 101],
            confidence: 0.92,
        }
    }
}

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

fn fixture_path(name: &str) -> PathBuf {
    samples_dir().join(name)
}

fn ingest_fixture(name: &str) -> String {
    let path = fixture_path(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

fn detect_from_step(content: &str) -> Vec<steprs_features::ManufacturingFeature> {
    let store = parse_step_file(content).expect("parse fixture");
    let cache = SchemaCache::build(&store);
    let brep = build_brep(&store, &cache);
    detect_coaxial_holes(&brep)
}

fn near_f64(a: f64, b: f64) -> bool {
    (a - b).abs() < EPS
}

fn near_f32(a: f32, b: f32) -> bool {
    (a - b).abs() < EPS as f32
}

fn assert_near_f32(label: &str, actual: f32, expected: f32) {
    assert!(
        near_f32(actual, expected),
        "{label}: expected {expected}, got {actual} (|Δ| = {})",
        (actual - expected).abs()
    );
}

fn assert_near_f64(label: &str, actual: f64, expected: f64) {
    assert!(
        near_f64(actual, expected),
        "{label}: expected {expected}, got {actual} (|Δ| = {})",
        (actual - expected).abs()
    );
}

fn assert_near_vec3(label: &str, actual: Vec3, expected: Vec3) {
    assert_near_f64(&format!("{label}.x"), actual.x, expected.x);
    assert_near_f64(&format!("{label}.y"), actual.y, expected.y);
    assert_near_f64(&format!("{label}.z"), actual.z, expected.z);
}

fn assert_near_slice(label: &str, actual: &[f64], expected: &[f64]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{label}: length mismatch (got {}, want {})",
        actual.len(),
        expected.len()
    );
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        assert_near_f64(&format!("{label}[{i}]"), *a, *e);
    }
}

fn sorted_face_ids(ids: &[u32]) -> Vec<u32> {
    let mut v: Vec<u32> = ids.to_vec();
    v.sort_unstable();
    v
}

fn assert_cylinder_block_cluster(
    cluster: &steprs_features::ManufacturingFeature,
    truth: &CylinderBlockTruth,
) {
    assert_eq!(
        cluster.kind,
        ManufacturingFeatureKind::BlindHole,
        "expected coaxial step bore (BlindHole)"
    );
    assert_eq!(
        sorted_face_ids(&cluster.face_ids),
        sorted_face_ids(&truth.face_ids),
        "face_ids mismatch"
    );

    let radius = cluster.radius.expect("radius");
    assert_near_f64("radius_mm", radius, truth.outer_radius_mm);

    let depth = cluster.depth.expect("depth");
    assert_near_f64("depth_mm", depth, truth.total_depth_mm);

    assert_near_vec3(
        "axis_origin",
        cluster.axis_origin.expect("axis_origin"),
        truth.axis_origin,
    );
    assert_near_vec3(
        "axis_direction",
        cluster.axis_direction.expect("axis_direction"),
        truth.axis_direction,
    );

    assert_near_f32("confidence", cluster.confidence, truth.confidence);

    let diameters = cluster
        .segment_diameters_mm
        .as_ref()
        .expect("segment_diameters_mm");
    assert_near_slice(
        "segment_diameters_mm",
        diameters,
        &[truth.outer_diameter_mm, truth.inner_diameter_mm],
    );

    let depths = cluster
        .segment_depths_mm
        .as_ref()
        .expect("segment_depths_mm");
    assert_near_slice("segment_depths_mm", depths, &truth.segment_depths_mm);

    assert_eq!(cluster.face_ids.len(), 2, "segment count");
    assert!(
        cluster.label.contains("Coaxial step bore"),
        "label should describe coaxial step bore: {}",
        cluster.label
    );
}

#[test]
fn differential_regression_ingests_all_fixtures() {
    for name in FIXTURES {
        let content = ingest_fixture(name);
        let holes = detect_from_step(&content);
        eprintln!("[ingest] {name}: {} coaxial cluster(s)", holes.len());
    }
}

#[test]
fn differential_regression_cylinder_block_dimensions() {
    let truth = CylinderBlockTruth::new();
    let content = ingest_fixture("cylinder_block.step");
    let holes = detect_from_step(&content);

    assert_eq!(
        holes.len(),
        1,
        "cylinder_block.step should yield exactly one coaxial cluster"
    );
    assert_cylinder_block_cluster(&holes[0], &truth);

    // Cross-check diameters against STEP radii (#10 r=12.7, #11 r=6.35).
    assert_near_f64(
        "outer diameter from STEP #10",
        holes[0].segment_diameters_mm.as_ref().unwrap()[0],
        truth.outer_radius_mm * 2.0,
    );
    assert_near_f64(
        "inner diameter from STEP #11",
        holes[0].segment_diameters_mm.as_ref().unwrap()[1],
        truth.inner_radius_mm * 2.0,
    );
    assert_near_f64(
        "z step between cylindrical origins",
        holes[0].segment_depths_mm.as_ref().unwrap()[0],
        truth.z_step_mm,
    );
}

#[test]
fn differential_regression_wasm_client_runtime_budget() {
    for name in ["cylinder_block.step", "rectangular_pocket.step"] {
        let content = ingest_fixture(name);
        let t0 = Instant::now();
        let json = analyze_web_json(&content).expect("analyze_web_json");
        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;

        eprintln!(
            "[profile] wasm-client pipeline ({name}): {elapsed_ms:.3} ms (budget {WASM_CLIENT_BUDGET_MS:.0} ms)"
        );

        assert!(
            !json.is_empty(),
            "wasm-client JSON should be non-empty for {name}"
        );
        assert!(
            json.contains("\"coaxial_holes\""),
            "wasm-client JSON missing coaxial_holes for {name}"
        );
        assert!(
            elapsed_ms < WASM_CLIENT_BUDGET_MS,
            "wasm-client processing for {name} exceeded {WASM_CLIENT_BUDGET_MS} ms: {elapsed_ms:.3} ms"
        );
    }
}

#[test]
fn differential_regression_cylinder_block_structural_summary() {
    let content = ingest_fixture("cylinder_block.step");
    let json = analyze_web_json(&content).expect("analyze_web_json");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("json parse");

    let summary = &parsed["structural_summary"];
    assert_eq!(summary["quantities"]["coaxial_step_bore"], 1);
    assert_eq!(summary["quantities"]["total_features"], 1);

    let feat = &summary["features"][0];
    assert_eq!(feat["classification"], "coaxial_step_bore");
    assert_eq!(feat["segment_quantity"], 2);

    let diameters: Vec<f64> = feat["diameters_mm"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_near_slice("manifest diameters_mm", &diameters, &[25.4, 12.7]);

    let depths: Vec<f64> = feat["segment_depths_mm"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_near_slice("manifest segment_depths_mm", &depths, &[10.0, 12.7]);

    let volume = feat["volume_limit_mm3"].as_f64().unwrap();
    let raw_volume = std::f64::consts::PI * 12.7_f64.powi(2) * 10.0
        + std::f64::consts::PI * 6.35_f64.powi(2) * 12.7;
    let expected_volume = (raw_volume * 1000.0).round() / 1000.0;
    assert_near_f64("volume_limit_mm3", volume, expected_volume);
}
