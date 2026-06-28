use steprs::{analyze_step, PipelineOptions};

#[test]
fn full_pipeline_on_sample() {
    let content = include_str!("../../../samples/cylinder_block.step");
    let result = analyze_step(
        content,
        &PipelineOptions {
            emit_gcode: true,
            ..Default::default()
        },
    )
    .expect("analyze");

    assert!(result.brep.face_count >= 2);
    assert!(!result.features.features.is_empty());
    assert!(result.toolpath.as_ref().unwrap().segments.len() > 0);
    assert!(!result.preview.cylinders.is_empty());
    assert!(result.gcode.as_ref().unwrap().contains("G21"));
}
