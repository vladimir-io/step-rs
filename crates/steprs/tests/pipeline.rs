use steprs::analyze_step;

#[test]
fn coaxial_pipeline_on_cylinder_block() {
    let content = include_str!("../../../samples/cylinder_block.step");
    let result = analyze_step(content).expect("analyze");

    assert!(result.brep.face_count >= 2);
    assert!(!result.coaxial_holes.is_empty(), "expected coaxial holes");
    assert!(
        result.coaxial_holes.iter().any(|f| f.face_ids.len() >= 2),
        "expected multi-face coaxial cluster"
    );
}
