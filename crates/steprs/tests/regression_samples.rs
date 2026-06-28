use steprs::{cylinder_block_spec, run_cylinder_block_regression, verify_sample};

#[test]
fn regression_cylinder_block() {
    let result = run_cylinder_block_regression();
    assert!(result.pass, "{}", result.detail);
    assert!(result.coaxial_count >= 1);
    assert!(result.max_segments >= 2);
}

#[test]
fn cylinder_block_coaxial_via_verify() {
    let content = include_str!("../../../samples/cylinder_block.step");
    let result = verify_sample(content, &cylinder_block_spec());
    assert!(result.pass, "{}", result.detail);
}
