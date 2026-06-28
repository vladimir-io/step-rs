use js_sys::{Array, Function};
use steprs::{
    analyze_web_json, analyze_web_json_with_progress, core::ParsePhase, core::ParseProgress,
    parse_only, PipelineOptions,
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

fn parse_options(options_json: &str) -> Result<PipelineOptions, JsValue> {
    match serde_json::from_str::<PipelineOptions>(options_json) {
        Ok(mut o) => {
            if !options_json.contains("emit_gcode") {
                o.emit_gcode = true;
            }
            Ok(o)
        }
        Err(e) => Err(JsValue::from_str(&format!("invalid options: {e}"))),
    }
}

#[wasm_bindgen(js_name = parseStep)]
pub fn parse_step(content: &str) -> Result<String, JsValue> {
    let stats = parse_only(content).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&stats).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = analyzeStep)]
pub fn analyze_step(content: &str, include_gcode: bool) -> Result<String, JsValue> {
    let options = PipelineOptions {
        emit_gcode: include_gcode,
        validate_gcode: include_gcode,
        simulate_stock: true,
        ..Default::default()
    };
    analyze_web_json(content, &options).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Full options JSON: `{ "emit_gcode": true, "tool": { "diameter_mm": 6 }, "post": { "processor": "fanuc" } }`
#[wasm_bindgen(js_name = analyzeStepOptions)]
pub fn analyze_step_options(content: &str, options_json: &str) -> Result<String, JsValue> {
    let options = parse_options(options_json)?;
    analyze_web_json(content, &options).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Same as analyzeStepOptions but invokes onProgress(phase, done, total) during parse/post.
#[wasm_bindgen(js_name = analyzeStepOptionsWithProgress)]
pub fn analyze_step_options_with_progress(
    content: &str,
    options_json: &str,
    on_progress: &Function,
) -> Result<String, JsValue> {
    let options = parse_options(options_json)?;
    let progress = JsProgress {
        callback: on_progress,
    };
    analyze_web_json_with_progress(content, &options, Some(&progress))
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
