use steprs::{analyze_step, analyze_web_json, PipelineOptions};

#[test]
fn web_json_omits_registry_and_is_smaller() {
    let content = include_str!("../../../samples/cylinder_block.step");
    let options = PipelineOptions {
        emit_gcode: true,
        stock_grid: 160,
        ..Default::default()
    };
    let full = analyze_step(content, &options).unwrap();
    let full_json = serde_json::to_string(&full).unwrap();
    let web_json = analyze_web_json(content, &options).unwrap();

    assert!(!web_json.contains("\"registry\""));
    assert!(web_json.contains("registry_summary"));
    assert!(web_json.len() < full_json.len());

    let parsed: serde_json::Value = serde_json::from_str(&web_json).unwrap();
    assert!(parsed.get("toolpath").is_some());
    assert!(parsed.get("gcode").is_some());
    assert!(parsed.get("mesh").is_some());
    let verts = parsed["mesh"]["vertices"].as_array().unwrap();
    assert!(!verts.is_empty());
}
