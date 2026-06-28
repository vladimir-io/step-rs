use steprs_core::parse_step_file;
use steprs_features::detect_coaxial_holes;
use steprs_schema::SchemaCache;
use steprs_topology::build_brep;

#[test]
fn detects_coaxial_cluster_on_cylinder_block() {
    let content = include_str!("../../../samples/cylinder_block.step");
    let store = parse_step_file(content).unwrap();
    let cache = SchemaCache::build(&store);
    let brep = build_brep(&store, &cache);
    let coaxial = detect_coaxial_holes(&brep);

    assert!(!coaxial.is_empty(), "expected coaxial holes");
    assert!(
        coaxial.iter().any(|f| f.face_ids.len() >= 2),
        "expected multi-face coaxial cluster"
    );
    let cluster = &coaxial[0];
    assert!(cluster.radius.unwrap_or(0.0) > 0.0);
    assert!(cluster.depth.unwrap_or(0.0) > 0.0);
    assert!(cluster.axis_direction.is_some());
}
