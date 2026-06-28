use std::fs;

use steprs_core::ManufacturingFeatureKind;
use steprs_core::RecordStore;
use steprs_schema::SchemaCache;
use steprs_topology::build_brep;
use steprs_features::{detect_planar_pockets, recognize_features};

fn load_sample(name: &str) -> RecordStore {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples")
        .join(name);
    let content = fs::read_to_string(path).unwrap();
    steprs_core::parse_step_file(&content).unwrap()
}

#[test]
fn detects_pocket_inner_loop() {
    let store = load_sample("rectangular_pocket.step");
    let cache = SchemaCache::build(&store);
    let brep = build_brep(&store, &cache);
    let pockets = detect_planar_pockets(&brep, &store, &cache);
    assert!(!pockets.is_empty());
    let p = &pockets[0];
    assert_eq!(p.kind, ManufacturingFeatureKind::RectangularPocket);
    assert!(p.pocket.as_ref().unwrap().corners.len() >= 3);
    assert!(p.confidence >= 0.85);
}

#[test]
fn pocket_profile_has_uv_axes() {
    let store = load_sample("rectangular_pocket.step");
    let cache = SchemaCache::build(&store);
    let brep = build_brep(&store, &cache);
    let model = recognize_features(&brep, &store, &cache);
    let pocket = model
        .features
        .iter()
        .find(|f| f.kind == ManufacturingFeatureKind::RectangularPocket)
        .expect("pocket feature");
    let profile = pocket.pocket.as_ref().unwrap();
    let u_len = (profile.u_axis[0].powi(2) + profile.u_axis[1].powi(2) + profile.u_axis[2].powi(2))
        .sqrt();
    assert!((u_len - 1.0).abs() < 0.01);
}
