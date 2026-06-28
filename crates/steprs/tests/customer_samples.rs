//! Regression tests on real AP214 customer STEP exports.
use std::fs;
use std::path::PathBuf;

use steprs::{customer_regression_specs, run_customer_regression, verify_sample};

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

#[test]
fn customer_samples_parse_and_analyze() {
    for expect in customer_regression_specs() {
        let path = samples_dir().join(&expect.name);
        let content = fs::read_to_string(&path).expect("read sample");
        let result = verify_sample(&content, &expect);
        assert!(result.pass, "{}: {}", expect.name, result.detail);
    }
}

#[test]
fn customer_regression_runner() {
    let cases = run_customer_regression();
    assert_eq!(cases.len(), customer_regression_specs().len());
    for case in cases {
        assert!(case.pass, "{}: {}", case.name, case.detail);
    }
}
