use steprs_core::{parse_stats, parse_step_file};

#[test]
fn parses_cylinder_sample() {
    let content = include_str!("../../../samples/cylinder_block.step");
    let store = parse_step_file(content).expect("parse");
    let stats = parse_stats(&store);
    assert!(stats.record_count >= 10);
    assert!(store.get(10).is_some());
    assert_eq!(store.get(10).unwrap().name, "CYLINDRICAL_SURFACE");
}
