use steprs::{analyze_step, analyze_web_json};

#[test]
fn web_json_omits_registry_and_coaxial_holes() {
    let content = include_str!("../../../samples/cylinder_block.step");
    let full = analyze_step(content).unwrap();
    let full_json = serde_json::to_string(&full).unwrap();
    let web_json = analyze_web_json(content).unwrap();

    assert!(!web_json.contains("\"registry\""));
    assert!(web_json.contains("registry_summary"));
    assert!(web_json.len() < full_json.len());

    let parsed: serde_json::Value = serde_json::from_str(&web_json).unwrap();
    assert!(parsed.get("coaxial_holes").is_some());
    assert!(parsed.get("structural_summary").is_some());
    assert!(parsed.get("toolpath").is_none());
    assert!(parsed.get("gcode").is_none());
    assert!(parsed.get("mesh").is_some());
    let holes = parsed["coaxial_holes"].as_array().unwrap();
    assert!(!holes.is_empty());
}
