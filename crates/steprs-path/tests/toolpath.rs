use steprs_core::parse_step_file;
use steprs_features::recognize_features;
use steprs_path::{
    emit_gcode_from_toolpath, job_to_string, plan_toolpath_brep, toolpath_from_gcode, PostOptions,
    ToolConfig,
};
use steprs_schema::SchemaCache;
use steprs_topology::build_brep;

#[test]
fn toolpath_roundtrip_via_gcode() {
    let content = include_str!("../../../samples/cylinder_block.step");
    let store = parse_step_file(content).unwrap();
    let cache = SchemaCache::build(&store);
    let brep = build_brep(&store, &cache);
    let features = recognize_features(&brep, &store, &cache);
    let tool = ToolConfig::default();
    let tp = plan_toolpath_brep(&features, Some(&brep), &tool);
    assert!(
        tp.stats.estimated_cut_length_mm < 80.0,
        "coaxial bore cut length should be plunge depth, not decorative arcs: {}",
        tp.stats.estimated_cut_length_mm
    );
    assert!(!tp.segments.is_empty());

    let post = PostOptions::default();
    let gcode = job_to_string(&emit_gcode_from_toolpath(&tp, &tool, &post));
    let parsed = toolpath_from_gcode(&gcode, tool.safe_z_mm);
    assert!(parsed.segments.len() >= tp.segments.len() / 2);
}
