use steprs_core::RecordStore;

/// Multiplier to convert file length units → millimetres.
pub fn detect_length_scale_to_mm(store: &RecordStore) -> f64 {
    let mut metre = false;
    let mut milli = false;

    for slot in &store.records {
        let Some(record) = slot else { continue };
        match record.name.as_str() {
            "CONVERSION_BASED_UNIT" => {
                if let Some(s) = record.param(0).and_then(|p| p.as_string()) {
                    let u = s.to_ascii_uppercase();
                    if u.contains("MILLI") {
                        milli = true;
                    } else if u.contains("METRE") || u.contains("METER") {
                        metre = true;
                    }
                }
            }
            "SI_UNIT" | "LENGTH_MEASURE_WITH_UNIT" => {
                for i in 0..record.parameters.len() {
                    if let Some(s) = record.param(i).and_then(|p| p.as_string()) {
                        let u = s.to_ascii_uppercase();
                        if u.contains("MILLI") {
                            milli = true;
                        } else if u.contains(".METRE.") || u == "METRE" || u.contains(".METER.") {
                            metre = true;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if milli {
        1.0
    } else if metre {
        1000.0
    } else {
        infer_scale_from_coordinates(store)
    }
}

fn infer_scale_from_coordinates(store: &RecordStore) -> f64 {
    let mut max_abs = 0.0f64;
    for slot in &store.records {
        let Some(record) = slot else { continue };
        if record.name != "CARTESIAN_POINT" {
            continue;
        }
        for i in 0..record.parameters.len() {
            if let Some(coords) = record.param(i).and_then(|p| p.as_vec3()) {
                for c in coords {
                    max_abs = max_abs.max(c.abs());
                }
            }
        }
    }
    if max_abs > 0.0 && max_abs < 8.0 {
        1000.0
    } else {
        1.0
    }
}

pub fn unit_label(scale: f64) -> &'static str {
    if (scale - 1000.0).abs() < f64::EPSILON {
        "m→mm"
    } else {
        "mm"
    }
}
