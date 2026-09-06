//! Test-only canonical cadence proof. Never restore a snapshot, normalized or otherwise.
use super::*;

fn continuation(snapshot: &[u8]) -> Result<String> {
    require(
        snapshot.len() == 320 && snapshot.starts_with(b"RSLC\x05\x0d\0\x01"),
        "expected canonical 320-byte schema5/profile13 snapshot",
    )?;
    let mut hashing_copy = snapshot.to_vec();
    hashing_copy[72..80].fill(0);
    Ok(sha256(&hashing_copy))
}

fn require(condition: bool, message: &str) -> Result<()> {
    if !condition {
        return Err(invalid(message).into());
    }
    Ok(())
}

fn non_clock(state: &Value) -> Value {
    let mut copy = state.clone();
    let fields = copy.as_object_mut().expect("GET state object");
    fields.remove("tick");
    fields.remove("snapshot_sha256");
    copy
}

fn equivalent(before: &Value, after: &Value) -> bool {
    before["continuation"] == after["continuation"]
        && non_clock(&before["state"]) == non_clock(&after["state"])
}

fn blocking(state: &Value) -> bool {
    !state["dialogue"].is_null() && state["dialogue_ready"] != false
}

fn omittable(command: u8, before: &Value, after: &Value) -> bool {
    command <= 4 && blocking(&before["state"]) && equivalent(before, after)
}

fn fixture(tick: u64, dialogue: Value, ready: bool) -> Value {
    json!({"state":{"tick":tick,"snapshot_sha256":"a".repeat(64),
        "dialogue":dialogue,"dialogue_ready":ready,"error":null,
        "start_kind":"new-game","x":304,"y":112,"map_id":15},
        "continuation":"b".repeat(64)})
}

#[test]
fn hashing_copy_changes_only_global_tick_and_never_the_input() {
    let mut bytes: Vec<u8> = (0..320).map(|n| (n % 251) as u8).collect();
    bytes[..8].copy_from_slice(b"RSLC\x05\x0d\0\x01");
    let original = bytes.clone();
    let mut expected = bytes.clone();
    expected[72..80].fill(0);
    let digest = continuation(&bytes).unwrap();
    assert_eq!(digest, sha256(&expected));
    assert_eq!(bytes, original);
    // Every bit outside the sole erased interval must affect identity, including
    // motion age/cursor, flags, coordinates, pot and reserved identity bytes.
    for index in 8..320 {
        for bit in 0..8 {
            let mut changed = bytes.clone();
            changed[index] ^= 1 << bit;
            assert_eq!(
                continuation(&changed).unwrap() == digest,
                (72..80).contains(&index),
                "byte {index} bit {bit}"
            );
        }
    }
}

#[test]
fn hashing_rejects_every_header_mutation_and_wrong_size() {
    let mut bytes = vec![0; 320];
    bytes[..8].copy_from_slice(b"RSLC\x05\x0d\0\x01");
    for index in 0..8 {
        for bit in 0..8 {
            let mut changed = bytes.clone();
            changed[index] ^= 1 << bit;
            assert!(continuation(&changed).is_err());
        }
    }
    for len in [0, 7, 181, 319, 321, 640] {
        let mut changed = bytes.clone();
        changed.resize(len, 0);
        assert!(continuation(&changed).is_err());
    }
}

#[test]
fn omission_requires_blocking_dialogue_and_all_nonclock_identity() {
    let before = fixture(8, json!({"key":"page"}), true);
    let mut after = before.clone();
    after["state"]["tick"] = json!(9);
    after["state"]["snapshot_sha256"] = json!("c".repeat(64));
    for command in 0..=10 {
        assert_eq!(omittable(command, &before, &after), command <= 4);
    }
    for key in ["x", "y", "map_id", "dialogue_ready", "dialogue", "error"] {
        let mut changed = after.clone();
        changed["state"][key] = json!("mutation");
        assert!(!omittable(0, &before, &changed), "{key}");
    }
    let mut changed = after.clone();
    changed["continuation"] = json!("d".repeat(64));
    assert!(!omittable(0, &before, &changed));
    for (dialogue, ready) in [(Value::Null, true), (json!({"key":"CEntry"}), false)] {
        let idle = fixture(8, dialogue, ready);
        assert!(!omittable(0, &idle, &idle));
    }
    let mut unknown = after.clone();
    unknown["state"]["new_future_field"] = json!(true);
    assert!(!omittable(0, &before, &unknown));
}

const ROUTE_SHA: &str = "b969d6877f595ff811a0de8a912de307aaf5a3eb2b42e1720f2f5da6db53b830";
const ROM_SHA: &str = "f331e3941e595cc41e26968c20b6e31563ad19603e5e204d93e3ee2e22344548";

fn hex(value: &Value, length: usize) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == length
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn emittable(command: u8, state: &Value) -> bool {
    let visible = !state["dialogue"].is_null();
    match command {
        0..=4 => !blocking(state) && (!visible || command == 0),
        5 | 10 => !visible,
        6 => blocking(state) && state["dialogue"].get("choice").is_none(),
        7..=9 => blocking(state) && state["dialogue"]["choice"].is_u64(),
        _ => false,
    }
}

fn boundaries<'a>(proof: &'a Value, name: &str) -> Result<&'a Vec<Value>> {
    let records = proof[name]
        .as_array()
        .ok_or_else(|| invalid("missing boundary array"))?;
    require(!records.is_empty(), "missing initial boundary")?;
    for (tick, record) in records.iter().enumerate() {
        let state = &record["state"];
        require(
            state.is_object()
                && state["tick"].as_u64() == Some(tick as u64)
                && state["start_kind"] == "new-game"
                && state.get("error") == Some(&Value::Null)
                && state.get("dialogue").is_some()
                && state["dialogue_ready"].is_boolean()
                && hex(&state["snapshot_sha256"], 64)
                && hex(&record["continuation"], 64),
            &format!("invalid {name} boundary {tick}; readiness relay is mandatory"),
        )?;
    }
    Ok(records)
}

fn validate_proof(commands: &[u8], proof: &Value) -> Result<Vec<usize>> {
    let provenance = &proof["provenance"];
    require(
        proof["schema"] == 1
            && provenance["route_sha256"] == ROUTE_SHA
            && provenance["rom_sha256"] == ROM_SHA
            && provenance["start"] == "NewGame"
            && provenance["policy"] == "SemanticPreview"
            && provenance["tick_erasure"] == "global-tick-only"
            && hex(&provenance["generator_revision"], 40)
            && hex(&provenance["source_tree"], 40)
            && matches!(
                provenance["build_profile"].as_str(),
                Some("debug" | "release")
            ),
        "invalid proof provenance/schema",
    )?;
    for field in [
        "source_archive_sha256",
        "compiler_content_sha256",
        "rustc_vv_sha256",
        "cargo_lock_sha256",
        "executable_sha256",
    ] {
        require(
            hex(&provenance[field], 64),
            &format!("invalid provenance {field}"),
        )?;
    }
    let offline = boundaries(proof, "offline")?;
    let projected = boundaries(proof, "projected")?;
    require(
        offline.len() == commands.len() + 1,
        "incomplete offline execution",
    )?;
    require(
        offline[0] == projected[0]
            && offline[0]["state"]["map_id"] == 15
            && offline[0]["state"]["x"] == 304
            && offline[0]["state"]["y"] == 112
            && offline[0]["state"]["dialogue"].is_null(),
        "initial fresh NewGame mismatch",
    )?;
    let mut omissions = Vec::new();
    let mut cursor = 1;
    for (index, &command) in commands.iter().enumerate() {
        if omittable(command, &offline[index], &offline[index + 1]) {
            omissions.push(index);
            continue;
        }
        require(
            emittable(command, &offline[index]["state"]),
            &format!("unemittable command {command} at offline {}", index + 1),
        )?;
        require(
            projected
                .get(cursor)
                .is_some_and(|boundary| equivalent(&offline[index + 1], boundary)),
            &format!(
                "projected retained boundary mismatch at offline {}",
                index + 1
            ),
        )?;
        cursor += 1;
    }
    require(cursor == projected.len(), "extra projected boundaries")?;
    Ok(omissions)
}

fn proof_fixture() -> (Vec<u8>, Value) {
    let initial = fixture(0, Value::Null, false);
    let page = fixture(1, json!({"key":"page"}), true);
    let mut paused = page.clone();
    paused["state"]["tick"] = json!(2);
    let resumed = fixture(3, Value::Null, false);
    let idle = fixture(4, Value::Null, false);
    let mut projected_resumed = resumed.clone();
    projected_resumed["state"]["tick"] = json!(2);
    let mut projected_idle = idle.clone();
    projected_idle["state"]["tick"] = json!(3);
    let mut provenance = json!({"route_sha256":ROUTE_SHA,"rom_sha256":ROM_SHA,
        "start":"NewGame","policy":"SemanticPreview","tick_erasure":"global-tick-only",
        "generator_revision":"a".repeat(40),"source_tree":"b".repeat(40),"build_profile":"debug"});
    for field in [
        "source_archive_sha256",
        "compiler_content_sha256",
        "rustc_vv_sha256",
        "cargo_lock_sha256",
        "executable_sha256",
    ] {
        provenance[field] = json!("c".repeat(64));
    }
    (
        vec![5, 2, 6, 0],
        json!({"schema":1,"provenance":provenance,
        "offline":[initial,page,paused,resumed,idle],
        "projected":[initial,page,projected_resumed,projected_idle]}),
    )
}

#[test]
fn proof_validator_requires_complete_fresh_replay_at_every_retained_boundary() {
    let (commands, proof) = proof_fixture();
    assert_eq!(validate_proof(&commands, &proof).unwrap(), vec![1]);
    for array in ["offline", "projected"] {
        for index in 0..proof[array].as_array().unwrap().len() {
            for field in ["tick", "dialogue_ready", "error", "start_kind"] {
                let mut bad = proof.clone();
                bad[array][index]["state"][field] = json!("invalid");
                assert!(
                    validate_proof(&commands, &bad).is_err(),
                    "{array} {index} {field}"
                );
            }
            let mut bad = proof.clone();
            bad[array].as_array_mut().unwrap().remove(index);
            assert!(validate_proof(&commands, &bad).is_err());
        }
        let mut bad = proof.clone();
        bad[array]
            .as_array_mut()
            .unwrap()
            .push(fixture(99, Value::Null, false));
        assert!(validate_proof(&commands, &bad).is_err());
    }
    for field in ["x", "unknown_future_field"] {
        let mut bad = proof.clone();
        bad["projected"][2]["state"][field] = json!(999);
        assert!(validate_proof(&commands, &bad).is_err());
    }
    let mut bad = proof.clone();
    bad["projected"][2]["continuation"] = json!("d".repeat(64));
    assert!(validate_proof(&commands, &bad).is_err());
    let mut bad = proof.clone();
    bad["offline"][1]["state"]["dialogue_ready"] = json!(false);
    bad["offline"][2]["state"]["dialogue_ready"] = json!(false);
    assert!(
        validate_proof(&commands, &bad).is_err(),
        "unfinished arrival cannot disappear"
    );
    for field in proof["provenance"].as_object().unwrap().keys() {
        let mut bad = proof.clone();
        bad["provenance"].as_object_mut().unwrap().remove(field);
        assert!(validate_proof(&commands, &bad).is_err(), "{field}");
    }
    for command in [6, 7, 10, 255] {
        let mut bad_commands = commands.clone();
        bad_commands[1] = command;
        assert!(validate_proof(&bad_commands, &proof).is_err());
    }
}

fn apply_raw(
    data: &GameData,
    state: &mut GameState,
    command: u8,
) -> std::result::Result<room_core::slice::FrameOutput, SliceError> {
    match command {
        5 => state.interact(data),
        6 => state.acknowledge(data),
        7..=9 => state.choose(data, command - 7),
        10 => state.pot_action(data),
        0..=4 => state.step(
            data,
            FrameInput {
                direction: match command {
                    1 => Some(Direction::Left),
                    2 => Some(Direction::Right),
                    3 => Some(Direction::Up),
                    4 => Some(Direction::Down),
                    _ => None,
                },
            },
        ),
        _ => Err(SliceError::Interaction),
    }
}

fn exact_raw_result(
    result: std::result::Result<room_core::slice::FrameOutput, SliceError>,
    expected: room_core::slice::FrameOutput,
) -> Result<()> {
    require(
        result == Ok(expected),
        &format!("raw core result differs: {result:?}; expected {expected:?}"),
    )
}

fn checked_step(preview: &mut Preview, raw: &mut GameState, command: u8) -> Result<()> {
    let result = apply_raw(&preview.data, raw, command);
    preview.step(command); // Actual public host command adapter, not a reconstructed host.
    exact_raw_result(result, preview.state.output())?;
    require(
        raw.snapshot() == preview.state.snapshot() && raw.output() == preview.state.output(),
        "host command differs from continuous raw core mirror",
    )?;
    require(preview.state()["error"].is_null(), "host projection error")
}

fn record(preview: &Preview) -> Result<Value> {
    let snapshot = preview.state.snapshot();
    let continuation = continuation(&snapshot)?;
    let state = preview.state(); // Exact GET projection, including unknown/new fields.
    require(
        state["error"].is_null()
            && state["dialogue_ready"].is_boolean()
            && state["dialogue_ready"]
                == json!(preview.state.dialogue_input_ready(&preview.data)?),
        "missing readiness relay or failed GET projection",
    )?;
    require(
        state["snapshot_sha256"] == sha256(&snapshot) && preview.state.snapshot() == snapshot,
        "GET mutated or misidentified core",
    )?;
    Ok(json!({"state":state,"continuation":continuation}))
}

fn fresh(rom: &Rom) -> Result<(Preview, GameState)> {
    let mut preview = Preview::new_profile(rom, true)?;
    preview.new_game();
    let raw = GameState::new_game(&preview.data, Policy::SemanticPreview);
    require(
        raw.snapshot() == preview.state.snapshot(),
        "fresh raw/host mismatch",
    )?;
    Ok((preview, raw))
}

fn route_commands(route: &Value) -> Result<Vec<u8>> {
    let actions = route["actions"]
        .as_array()
        .ok_or_else(|| invalid("route actions"))?;
    let mut commands = Vec::new();
    for action in actions {
        let command = action[0].as_u64().ok_or_else(|| invalid("route command"))?;
        let count = action[1].as_u64().ok_or_else(|| invalid("route count"))?;
        require(
            action.as_array().is_some_and(|a| a.len() == 2)
                && command <= 10
                && count > 0
                && count <= 11590,
            "invalid route span",
        )?;
        commands.extend(std::iter::repeat_n(
            u8::try_from(command)?,
            usize::try_from(count)?,
        ));
    }
    require(
        commands.len() == 11590,
        "route must expand to 11590 actions",
    )?;
    Ok(commands)
}

#[test]
#[ignore = "private ROM + clean-source wrapper required; never skip missing evidence"]
fn generate_canonical_cadence_proof() -> Result<()> {
    use std::{env, fs, io::Write};
    let route_bytes = include_bytes!("../../../../tools/pandora-runtime-qualification/route.json");
    require(
        sha256(route_bytes) == ROUTE_SHA,
        "fixed route file hash differs",
    )?;
    let route: Value = serde_json::from_slice(route_bytes)?;
    require(route["rom_sha256"] == ROM_SHA, "route ROM identity differs")?;
    let commands = route_commands(&route)?;
    let rom_bytes =
        fs::read(env::var_os("PANDORA_CADENCE_ROM").ok_or_else(|| invalid("ROM required"))?)?;
    let rom = Rom::load(&rom_bytes)?;
    require(sha256(rom.image()) == ROM_SHA, "ROM identity differs")?;
    let mut provenance: Value = serde_json::from_slice(&fs::read(
        env::var_os("PANDORA_CADENCE_PROVENANCE")
            .ok_or_else(|| invalid("wrapper provenance required"))?,
    )?)?;
    let (mut original, mut raw) = fresh(&rom)?;
    let initial_snapshot = original.state.snapshot();
    let mut offline = vec![record(&original)?];
    provenance["compiler_content_sha256"] = json!(initial_snapshot[40..72]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>());
    provenance["rom_sha256"] = json!(ROM_SHA);
    provenance["route_sha256"] = json!(ROUTE_SHA);
    provenance["start"] = json!("NewGame");
    provenance["policy"] = json!("SemanticPreview");
    provenance["tick_erasure"] = json!("global-tick-only");
    for (index, &command) in commands.iter().enumerate() {
        checked_step(&mut original, &mut raw, command)
            .map_err(|error| invalid(&format!("offline {}: {error}", index + 1)))?;
        offline.push(record(&original)?);
    }
    require(
        offline.last().unwrap()["state"]["snapshot_sha256"] == route["expected"]["snapshot_sha256"],
        "unprojected route final canonical identity changed",
    )?;

    // A second independently compiled host, NewGame and continuous raw mirror.
    // Neither offline state nor snapshot bytes ever initialize this branch.
    let (mut replay, mut replay_raw) = fresh(&rom)?;
    let mut projected = vec![record(&replay)?];
    let mut arrivals = BTreeMap::<String, usize>::new();
    for (index, &command) in commands.iter().enumerate() {
        let before = &offline[index];
        let after = &offline[index + 1];
        if omittable(command, before, after) {
            continue;
        }
        require(
            emittable(command, &before["state"]),
            &format!("cannot emit offline command {}", index + 1),
        )?;
        if command <= 4 && !before["state"]["dialogue"].is_null() {
            require(
                before["state"]["dialogue_ready"] == false && !equivalent(before, after),
                "visible unfinished arrival must advance real continuation",
            )?;
            let invocation = before["state"]["pandora"]["invocation"]
                .as_str()
                .ok_or_else(|| invalid("missing arrival invocation"))?;
            *arrivals.entry(invocation.into()).or_default() += 1;
        }
        checked_step(&mut replay, &mut replay_raw, command)
            .map_err(|error| invalid(&format!("projected at offline {}: {error}", index + 1)))?;
        let boundary = record(&replay)?;
        require(
            equivalent(after, &boundary),
            &format!("fresh replay differs at offline {}", index + 1),
        )?;
        projected.push(boundary);
    }
    let mut proof =
        json!({"schema":1,"provenance":provenance,"offline":offline,"projected":projected});
    let omissions = validate_proof(&commands, &proof)?;
    require(
        omissions.len() == 181,
        "qualified no-op count differs (computed, never assumed)",
    )?;
    require(
        arrivals == BTreeMap::from([("CEntry".into(), 17), ("BoxEntry".into(), 1)]),
        "must retain all 17 CEntry arrival updates and the BoxEntry completion",
    )?;
    proof["qualification"] = json!({"original_actions":commands.len(),
        "projected_actions":proof["projected"].as_array().unwrap().len()-1,
        "omitted_indices":omissions,"retained_visible_arrival_updates":arrivals,
        "raw_core_results":"all-success-exact-host-match"});
    // The wrapper publishes only after this test exits successfully and rechecks
    // source identity. A failed attempt can leave diagnostics, never an accepted proof.
    let output =
        env::var_os("PANDORA_CADENCE_PENDING").ok_or_else(|| invalid("pending output required"))?;
    let bytes = serde_json::to_vec(&proof)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    println!("cadence validated: 11590 original, {} projected, 181 omitted, 18 retained arrivals; sha256 {}",
        proof["qualification"]["projected_actions"], sha256(&bytes));
    Ok(())
}

#[test]
#[ignore = "private ROM required; exercises host suppression of Interaction"]
fn raw_rejection_cannot_be_hidden_by_host_noop() -> Result<()> {
    let rom = Rom::load(&std::fs::read(
        std::env::var_os("PANDORA_CADENCE_ROM").ok_or_else(|| invalid("ROM required"))?,
    )?)?;
    let (mut preview, mut raw) = fresh(&rom)?;
    let before = preview.state.snapshot();
    assert_eq!(
        apply_raw(&preview.data, &mut raw, 6),
        Err(SliceError::Interaction)
    );
    assert!(checked_step(&mut preview, &mut raw, 6).is_err());
    assert_eq!(preview.state.snapshot(), before);
    assert_eq!(preview.state()["error"], Value::Null);
    Ok(())
}

#[test]
fn raw_success_must_return_the_exact_frame_not_merely_ok() {
    let expected = room_core::slice::FrameOutput {
        tick: 1,
        map_id: 15,
        position: (304, 112),
        phase: Phase::Walking,
        animation: room_core::AnimationFrame {
            set: room_core::AnimationSet::Standing,
            sequence: 0,
            record: 0,
            mirror_x: false,
        },
    };
    assert!(exact_raw_result(Ok(expected), expected).is_ok());
    assert!(exact_raw_result(Err(SliceError::Interaction), expected).is_err());
    let mut wrong = expected;
    wrong.tick += 1;
    assert!(exact_raw_result(Ok(wrong), expected).is_err());
    wrong = expected;
    wrong.position.0 += 1;
    assert!(exact_raw_result(Ok(wrong), expected).is_err());
}
