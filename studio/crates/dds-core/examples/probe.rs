//! CLI sanity probe of the models: `cargo run -p dds-core --example probe`
use dds_core::Engine;
use serde_json::json;

fn main() {
    let mut e = Engine::new(None);
    let s = e.handle("get_state", json!({})).expect("state");
    for m in s["derived"]["metrics"].as_array().unwrap() {
        println!("{:<18} {:>14} {}", m["label"].as_str().unwrap(), m["value"], m["unit"].as_str().unwrap());
    }
    for i in s["derived"]["issues"].as_array().unwrap() {
        println!("[{}] {}", i["level"].as_str().unwrap(), i["text"].as_str().unwrap());
    }
    let sp = e.handle("spectrum", json!({ "lo": 0, "hi": 1e9 })).expect("spectrum");
    for t in sp["tones"].as_array().unwrap() {
        println!("{:<44} {:>12.3} МГц {:>8.1} дБм {:>7.1} дБн", t["name"].as_str().unwrap(), t["f"].as_f64().unwrap() / 1e6, t["dbm"].as_f64().unwrap(), t["dbc"].as_f64().unwrap());
    }
}
