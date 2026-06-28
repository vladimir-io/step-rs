use steprs_core::parse_step_file;
use steprs_core::parser::entity::entity_instance;
use steprs_core::parser::parameter::{parameter, parse_parameter_list};

#[test]
fn parses_ref_list_param() {
    parameter(" ( #10, #11, #12 )").expect("list param");
}

#[test]
fn parses_ap214_param_list() {
    let inner = " ' ', ( #10, #11, #12 ), #6 ";
    let (rest, params) = parse_parameter_list(inner).expect("params");
    assert_eq!(params.len(), 3, "rest: {rest}");
    assert!(rest.trim().is_empty());
}

#[test]
fn parses_ap214_entity_one() {
    let line = "#1 = MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION( ' ', ( #10, #11, #12, #13, #14, #15, #16, #17, #18, #19, #20, #21, #22, #23, #24, #25, #26, #27, #28, #29, #30, #31, #32, #33 ), #6 );";
    let (rest, inst) = entity_instance(line).expect("entity #1");
    assert!(rest.trim().is_empty());
    assert_eq!(inst.id(), 1);
}

#[test]
fn parses_ap214_complex_entity_six() {
    let line = "#6 =  ( GEOMETRIC_REPRESENTATION_CONTEXT( 3 )GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT( ( #41 ) )GLOBAL_UNIT_ASSIGNED_CONTEXT( ( #43, #44, #45 ) )REPRESENTATION_CONTEXT( 'NONE', 'WORKSPACE' ) );";
    let (rest, inst) = entity_instance(line).expect("entity #6");
    assert!(rest.trim().is_empty());
    assert_eq!(inst.id(), 6);
}

#[test]
fn parses_ap214_fixture_header_and_data() {
    let content = include_str!("../../../samples/ap214_compact.step");
    let store = parse_step_file(content).expect("ap214 fixture");
    assert!(
        store.record_count() > 500,
        "records: {}",
        store.record_count()
    );
}
