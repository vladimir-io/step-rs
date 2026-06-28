//! Regression tests on anonymized AP214 export fixtures.

use std::fs;
use std::path::PathBuf;
use steprs::{ap214_regression_specs, run_ap214_regression, verify_sample};

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

#[test]
fn ap214_fixtures_parse_and_analyze() {
    for expect in ap214_regression_specs() {
        let path = samples_dir().join(&expect.name);
        let content = fs::read_to_string(&path).expect("read sample");
        let result = verify_sample(&content, &expect);
        assert!(result.pass, "{}: {}", expect.name, result.detail);
    }
}

#[test]
fn ap214_regression_runner() {
    let cases = run_ap214_regression();
    assert_eq!(cases.len(), ap214_regression_specs().len());
    for case in cases {
        assert!(case.pass, "{}: {}", case.name, case.detail);
    }
}
