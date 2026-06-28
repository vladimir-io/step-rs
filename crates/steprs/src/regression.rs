//! Shared regression checks for cylinder_block and customer AP214 samples.

use serde::{Deserialize, Serialize};
use steprs_core::parse_step_file;
use steprs_features::detect_coaxial_holes;
use steprs_schema::SchemaCache;
use steprs_topology::build_brep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleExpect {
    pub name: String,
    pub min_records: usize,
    pub min_faces: usize,
    pub protocol: String,
    /// Require at least one coaxial cluster (2+ cylindrical faces).
    pub require_coaxial_cluster: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifyResult {
    pub name: String,
    pub pass: bool,
    pub detail: String,
    pub record_count: usize,
    pub face_count: usize,
    pub coaxial_count: usize,
    pub max_segments: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemTestStatus {
    pub all_pass: bool,
    pub cases: Vec<VerifyResult>,
}

pub fn cylinder_block_spec() -> SampleExpect {
    SampleExpect {
        name: "cylinder_block.step".into(),
        min_records: 10,
        min_faces: 2,
        protocol: "AP203".into(),
        require_coaxial_cluster: true,
    }
}

pub fn customer_regression_specs() -> Vec<SampleExpect> {
    vec![
        SampleExpect {
            name: "customer_00167362.step".into(),
            min_records: 1800,
            min_faces: 40,
            protocol: "AP214".into(),
            require_coaxial_cluster: false,
        },
        SampleExpect {
            name: "customer_00013700.step".into(),
            min_records: 11_000,
            min_faces: 150,
            protocol: "AP214".into(),
            require_coaxial_cluster: false,
        },
        SampleExpect {
            name: "customer_00180964.step".into(),
            min_records: 700,
            min_faces: 20,
            protocol: "AP214".into(),
            require_coaxial_cluster: false,
        },
        SampleExpect {
            name: "customer_00144025.step".into(),
            min_records: 1500,
            min_faces: 30,
            protocol: "AP214".into(),
            require_coaxial_cluster: false,
        },
    ]
}

pub fn verify_sample(content: &str, expect: &SampleExpect) -> VerifyResult {
    let fail = |detail: String| VerifyResult {
        name: expect.name.clone(),
        pass: false,
        detail,
        record_count: 0,
        face_count: 0,
        coaxial_count: 0,
        max_segments: 0,
    };

    let store = match parse_step_file(content) {
        Ok(s) => s,
        Err(e) => return fail(format!("parse: {e}")),
    };

    let stats = steprs_core::parse_stats(&store);
    if stats.application_protocol != expect.protocol {
        return fail(format!(
            "protocol {} (expected {})",
            stats.application_protocol, expect.protocol
        ));
    }
    if stats.record_count < expect.min_records {
        return fail(format!(
            "records {} < {}",
            stats.record_count, expect.min_records
        ));
    }

    let cache = SchemaCache::build(&store);
    let brep = build_brep(&store, &cache);
    if brep.faces.len() < expect.min_faces {
        return fail(format!(
            "faces {} < {}",
            brep.faces.len(),
            expect.min_faces
        ));
    }

    let coaxial = detect_coaxial_holes(&brep);
    let max_segments = coaxial
        .iter()
        .map(|f| f.face_ids.len())
        .max()
        .unwrap_or(0);

    if expect.require_coaxial_cluster {
        if coaxial.is_empty() {
            return fail("no coaxial holes detected".to_string());
        }
        if max_segments < 2 {
            return fail(format!("max segments {max_segments} < 2"));
        }
    }

    let detail = if coaxial.is_empty() {
        format!(
            "{} · {} records · {} faces · coaxial: 0",
            stats.application_protocol,
            stats.record_count,
            brep.faces.len()
        )
    } else {
        format!(
            "{} · {} records · {} faces · coaxial: {} (max {} seg)",
            stats.application_protocol,
            stats.record_count,
            brep.faces.len(),
            coaxial.len(),
            max_segments
        )
    };

    VerifyResult {
        name: expect.name.clone(),
        pass: true,
        detail,
        record_count: stats.record_count,
        face_count: brep.faces.len(),
        coaxial_count: coaxial.len(),
        max_segments,
    }
}

pub fn run_cylinder_block_regression() -> VerifyResult {
    let content = include_str!("../../../samples/cylinder_block.step");
    verify_sample(content, &cylinder_block_spec())
}

pub fn run_customer_regression() -> Vec<VerifyResult> {
    customer_regression_specs()
        .iter()
        .map(|spec| {
            let path = format!(
                "{}/../../samples/{}",
                env!("CARGO_MANIFEST_DIR"),
                spec.name
            );
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", spec.name));
            verify_sample(&content, spec)
        })
        .collect()
}

pub fn run_system_tests() -> SystemTestStatus {
    let mut cases = vec![run_cylinder_block_regression()];
    cases.extend(run_customer_regression());
    let all_pass = cases.iter().all(|c| c.pass);
    SystemTestStatus { all_pass, cases }
}

/// Specs for browser-side customer regression (fetch + verify).
pub fn customer_specs_json() -> Result<String, serde_json::Error> {
    serde_json::to_string(&customer_regression_specs())
}
