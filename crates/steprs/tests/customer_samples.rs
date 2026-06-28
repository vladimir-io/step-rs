//! Regression tests on real AP214 customer STEP exports.
use std::fs;
use std::path::PathBuf;

use steprs::{analyze_step, PipelineOptions};

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

struct CustomerExpect {
    name: &'static str,
    min_records: usize,
    min_faces: usize,
    min_features: usize,
    protocol: &'static str,
}

const CUSTOMERS: &[CustomerExpect] = &[
    CustomerExpect {
        name: "customer_00167362.step",
        min_records: 1800,
        min_faces: 40,
        min_features: 1,
        protocol: "AP214",
    },
    CustomerExpect {
        name: "customer_00013700.step",
        min_records: 11_000,
        min_faces: 150,
        min_features: 1,
        protocol: "AP214",
    },
    CustomerExpect {
        name: "customer_00180964.step",
        min_records: 700,
        min_faces: 20,
        min_features: 1,
        protocol: "AP214",
    },
    CustomerExpect {
        name: "customer_00144025.step",
        min_records: 1500,
        min_faces: 30,
        min_features: 1,
        protocol: "AP214",
    },
];

#[test]
fn customer_samples_parse_and_analyze() {
    for expect in CUSTOMERS {
        let path = samples_dir().join(expect.name);
        let content = fs::read_to_string(&path).expect("read sample");
        let result = analyze_step(&content, &PipelineOptions::default())
            .unwrap_or_else(|e| panic!("{}: analyze failed: {e}", expect.name));

        assert_eq!(
            result.stats.application_protocol, expect.protocol,
            "{}",
            expect.name
        );
        assert!(
            result.stats.record_count >= expect.min_records,
            "{}: records {}",
            expect.name,
            result.stats.record_count
        );
        assert!(
            result.brep.face_count >= expect.min_faces,
            "{}: faces {}",
            expect.name,
            result.brep.face_count
        );
        assert!(
            result.features.features.len() >= expect.min_features,
            "{}: features {}",
            expect.name,
            result.features.features.len()
        );
        assert!(
            result.toolpath.as_ref().map(|t| t.segments.len()).unwrap_or(0) > 0,
            "{}: expected toolpath",
            expect.name
        );
    }
}
