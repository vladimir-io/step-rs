use steprs_path::{emit_gcode, validate_job, ToolConfig};
use steprs_features::{FeatureModel, FeatureSummary};

#[test]
fn validates_emitted_job() {
    let model = FeatureModel {
        features: vec![],
        summary: FeatureSummary::default(),
    };
    let job = emit_gcode(&model, &ToolConfig::default());
    let v = validate_job(&job);
    assert!(v.warnings.len() >= 1 || v.valid);
}
