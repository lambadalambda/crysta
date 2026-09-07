//! Produce native expectations for the bounded actual-Wasm parity replay.
use pandora_web::Session;
use serde_json::{json, Value};

fn digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    rom::digests(bytes)
        .sha256
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("writing to String");
            hex
        })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: cargo run -p pandora-web --example parity -- ROM")?;
    let mut session = Session::new(&std::fs::read(path)?)?;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tools/pandora-preview/parity-actions.json"
    ))?;
    session.new_game();
    let mut inputs = Vec::new();
    for run in fixture["actions"].as_array().ok_or("actions array")? {
        let input = u8::try_from(run[0].as_u64().ok_or("input")?)?;
        let count = usize::try_from(run[1].as_u64().ok_or("count")?)?;
        inputs.extend(std::iter::repeat_n(input, count));
    }
    let state = session.run_parity_inputs(&inputs)?;
    let visible_unready = serde_json::from_str::<Value>(&state)?;
    let after_ack = serde_json::from_str::<Value>(&session.step(6)?)?;
    let after_neutral = serde_json::from_str::<Value>(&session.step(0)?)?;
    let backgrounds = ["house", "exterior", "town13", "cellars", "box", "tour"]
        .into_iter()
        .map(|key| Ok((key, digest(&session.background(key)?))))
        .collect::<Result<std::collections::BTreeMap<_, _>, String>>()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema":1,"commands":inputs.len(),
            "states":{"visible_unready":visible_unready,"after_ack":after_ack,"after_neutral":after_neutral},
            "art_sha256":digest(&session.art()),"background_sha256":backgrounds
        }))?
    );
    Ok(())
}
