use js_sys::{Array, Function};
use steprs::{
    analyze_web_json, analyze_web_json_with_progress, core::ParsePhase, core::ParseProgress,
    parse_only, regression_specs_json, run_cylinder_block_regression, verify_sample,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

struct JsProgress<'a> {
    callback: &'a Function,
}

impl ParseProgress for JsProgress<'_> {
    fn on_phase(&self, phase: ParsePhase, done: usize, total: Option<usize>) {
        let phase_name = match phase {
            ParsePhase::Header => "header",
            ParsePhase::DataScan => "data_scan",
            ParsePhase::Indexing => "indexing",
            ParsePhase::Topology => "topology",
            ParsePhase::Features => "features",
            ParsePhase::Toolpath => "toolpath",
            ParsePhase::PostProcess => "post_process",
            ParsePhase::StockSim => "stock_sim",
        };
        let args = Array::new();
        args.push(&JsValue::from_str(phase_name));
        args.push(&JsValue::from(done as u32));
        args.push(&match total {
            Some(t) => JsValue::from(t as u32),
            None => JsValue::NULL,
        });
        let _ = self.callback.apply(&JsValue::NULL, &args);
    }
}

#[wasm_bindgen(js_name = parseStep)]
pub fn parse_step(content: &str) -> Result<String, JsValue> {
    let stats = parse_only(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&stats).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = analyzeStep)]
pub fn analyze_step(content: &str) -> Result<String, JsValue> {
    analyze_web_json(content).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Legacy alias — options JSON is ignored.
#[wasm_bindgen(js_name = analyzeStepOptions)]
pub fn analyze_step_options(content: &str, _options_json: &str) -> Result<String, JsValue> {
    analyze_web_json(content).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = analyzeStepOptionsWithProgress)]
pub fn analyze_step_options_with_progress(
    content: &str,
    _options_json: &str,
    on_progress: &Function,
) -> Result<String, JsValue> {
    let progress = JsProgress {
        callback: on_progress,
    };
    analyze_web_json_with_progress(content, Some(&progress))
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = verifyCylinderBlockRegression)]
pub fn verify_cylinder_block_regression() -> Result<String, JsValue> {
    let result = run_cylinder_block_regression();
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = verifyRegressionSample)]
pub fn verify_regression_sample(content: &str, spec_json: &str) -> Result<String, JsValue> {
    let spec = serde_json::from_str(spec_json)
        .map_err(|e| JsValue::from_str(&format!("invalid spec: {e}")))?;
    let result = verify_sample(content, &spec);
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = regressionSpecs)]
pub fn regression_specs() -> Result<String, JsValue> {
    regression_specs_json().map_err(|e| JsValue::from_str(&e.to_string()))
}
