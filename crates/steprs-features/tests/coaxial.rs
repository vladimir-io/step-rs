use steprs_core::parse_step_file;
use steprs_features::recognize_features;
use steprs_schema::SchemaCache;
use steprs_topology::build_brep;

#[test]
fn detects_cylindrical_features_on_sample() {
    let content = include_str!("../../../samples/cylinder_block.step");
    let store = parse_step_file(content).unwrap();
    let cache = SchemaCache::build(&store);
    let brep = build_brep(&store, &cache);
    let model = recognize_features(&brep, &store, &cache);

    assert!(!model.features.is_empty());
    let has_hole = model.summary.hole_count > 0;
    let has_cyl = model.summary.cylindrical_face_count > 0;
    assert!(has_hole || has_cyl);
}
