use crate::core::{parse_config, run_batch, FoundResult, MinerConfig, RawConfig};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
struct BatchResult {
    found: Vec<FoundResult>,
    attempts: u32,
}

#[derive(Debug)]
struct WorkerState {
    config: Option<MinerConfig>,
    seed: u64,
    worker_id: u32,
    stop: bool,
}

thread_local! {
    static STATE: RefCell<WorkerState> = RefCell::new(WorkerState {
        config: None,
        seed: 0,
        worker_id: 0,
        stop: false,
    });
}

#[wasm_bindgen]
pub fn init_worker(config: JsValue, seed: u32, worker_id: u32) -> Result<(), JsValue> {
    let raw: RawConfig = serde_wasm_bindgen::from_value(config)
        .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;
    let parsed = parse_config(raw).map_err(|e| JsValue::from_str(&e))?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.config = Some(parsed);
        state.seed = seed as u64;
        state.worker_id = worker_id;
        state.stop = false;
    });

    Ok(())
}

#[wasm_bindgen]
pub fn set_stop(flag: bool) {
    STATE.with(|state| {
        state.borrow_mut().stop = flag;
    });
}

#[wasm_bindgen]
pub fn run_batch_wasm(batch_size: u32) -> Result<JsValue, JsValue> {
    let result = STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.stop {
            return Ok::<BatchResult, JsValue>(BatchResult {
                found: vec![],
                attempts: 0,
            });
        }

        let config = state
            .config
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Worker not initialized"))?;

        let (found, attempts) = run_batch(config, state.seed, state.worker_id, batch_size);
        state.seed = state.seed.wrapping_add(1);

        Ok(BatchResult { found, attempts })
    })?;

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}
