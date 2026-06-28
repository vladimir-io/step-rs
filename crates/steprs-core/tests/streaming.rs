use steprs_core::parse_step_streaming;

#[test]
fn fail_soft_skips_bad_entity() {
    let content = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('AP203'));
ENDSEC;
DATA;
#1 = CARTESIAN_POINT('',(0.0,0.0,0.0));
#2 = THIS_IS_NOT_VALID syntax {{{;
#3 = CARTESIAN_POINT('',(1.0,0.0,0.0));
ENDSEC;
END-ISO-10303-21;
"#;
    let store = parse_step_streaming(content, None).expect("parse");
    assert_eq!(store.record_count(), 2);
    assert_eq!(store.parse_error_count(), 1);
}
