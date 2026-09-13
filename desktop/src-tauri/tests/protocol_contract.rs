use std::fs;
use std::path::{Path, PathBuf};

use jsonschema::Validator;
use serde_json::{Map, Value};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri is three levels below repo root")
        .to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    repo_root().join("protocol").join("fixtures")
}

fn load_json(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e))
}

fn compiled_validator() -> Validator {
    let schema = load_json(
        &repo_root()
            .join("protocol")
            .join("control-message.schema.json"),
    );
    Validator::new(&schema).expect("frozen schema must compile")
}

#[test]
fn every_valid_fixture_passes_schema() {
    let validator = compiled_validator();
    let dir = fixtures_dir().join("valid");
    let mut count = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let doc = load_json(&path);
        if let Err(err) = validator.validate(&doc) {
            panic!("{} rejected by schema: {}", path.display(), err);
        }
        count += 1;
    }
    assert!(count >= 10, "expected >=10 valid fixtures, found {}", count);
}

#[test]
fn every_invalid_fixture_is_rejected_by_schema() {
    let validator = compiled_validator();
    let dir = fixtures_dir().join("invalid");
    let mut count = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let doc = load_json(&path);
        assert!(
            !validator.is_valid(&doc),
            "{}: schema accepted a fixture declared invalid",
            path.display()
        );
        count += 1;
    }
    assert!(
        count >= 15,
        "expected >=15 invalid fixtures, found {}",
        count
    );
}

#[test]
fn large_request_ids_round_trip_exactly_in_u64_and_js_safe_bounds() {
    let max_safe: u64 = 9007199254740991;
    let doc = serde_json::json!({
        "v": 1, "id": max_safe, "type": "ping",
        "payload": { "sessionGeneration": 0 }
    });
    assert!(
        compiled_validator().is_valid(&doc),
        "max JSON-safe id must validate"
    );
    let parsed: u64 = doc["id"].as_u64().expect("id parses losslessly as u64");
    assert_eq!(parsed, max_safe);

    let beyond = serde_json::json!({
        "v": 1, "id": max_safe + 1, "type": "ping",
        "payload": { "sessionGeneration": 0 }
    });
    assert!(
        !compiled_validator().is_valid(&beyond),
        "id above safe range must reject"
    );
}

#[test]
fn units_are_exact_integers_where_contract_declares_them() {
    let doc = load_json(&fixtures_dir().join("valid").join("camera.set.json"));
    let steps = doc["payload"]["changes"]["exposureCompensationSteps"]
        .as_i64()
        .expect("EV steps are integer");
    assert_eq!(steps, 2);
    let caps = load_json(&fixtures_dir().join("valid").join("capabilities.json"));
    let shutter = caps["payload"]["cameras"][0]["controls"]["shutterNanos"]["max"]
        .as_i64()
        .expect("shutter nanoseconds are integer");
    assert_eq!(shutter, 33_333_333);
}

fn semantic_expect(case: &Map<String, Value>) -> &'static str {
    let name = case["case"].as_str().expect("semantic case name");
    let ctx = case.get("context").cloned().unwrap_or(Value::Null);
    let messages = case["messages"].as_array().expect("messages");
    match name {
        "stale_capabilities_revision" => {
            let rev = messages[0]["payload"]["capabilityRevision"]
                .as_u64()
                .unwrap();
            let cur = ctx["currentCapabilityRevision"].as_u64().unwrap();
            if rev == cur {
                "cache_replay"
            } else {
                "reject"
            }
        }
        "out_of_range_ev_steps" => {
            let rng = &ctx["capabilities"]["exposureCompensation"];
            let steps = messages[0]["payload"]["changes"]["exposureCompensationSteps"]
                .as_i64()
                .unwrap();
            let min = rng["min"].as_i64().unwrap();
            let max = rng["max"].as_i64().unwrap();
            if (min..=max).contains(&steps) {
                "cache_replay"
            } else {
                "reject"
            }
        }
        "manual_exposure_not_atomic" | "partial_manual_exposure" => {
            let changes = messages[0]["payload"]["changes"].as_object().unwrap();
            let ae_off = changes.get("aeEnabled").and_then(|v| v.as_bool()) == Some(false);
            let full_manual =
                ae_off && changes.contains_key("iso") && changes.contains_key("shutterNanos");
            let manual_touched =
                changes.contains_key("iso") || changes.contains_key("shutterNanos");
            if manual_touched && !full_manual {
                "reject"
            } else {
                "cache_replay"
            }
        }
        "duplicate_id_same_content" | "duplicate_id_changed_content" => {
            let (first, second) = (&messages[0], &messages[1]);
            let same_id = first["id"] == second["id"];
            if same_id && first == second {
                "cache_replay"
            } else if same_id {
                "reject"
            } else {
                "cache_replay"
            }
        }
        "stale_generation_event" => {
            let gen = messages[0]["payload"]["connectionGeneration"]
                .as_u64()
                .unwrap();
            let active = ctx["activeConnectionGeneration"].as_u64().unwrap();
            if gen == active {
                "cache_replay"
            } else {
                "ignore"
            }
        }
        "wrong_direction_request" => {
            if ctx["observedDirection"].as_str() == Some("desktop_to_phone") {
                "cache_replay"
            } else {
                "reject"
            }
        }
        "oversized_control_message" => {
            let blob = serde_json::to_string(&messages[0]).unwrap();
            if blob.len() > 65536 {
                "reject"
            } else {
                "cache_replay"
            }
        }
        "pairing-secret-replay"
        | "pairing-secret-expired"
        | "unauthenticated-media-request"
        | "revoked-peer-control-connection" => {
            // Trust semantics: fail closed on replay/expiry/unauthenticated/revoked.
            match name {
                "pairing-secret-replay" if ctx["pairingState"].as_str() == Some("consumed") => {
                    "reject"
                }
                "pairing-secret-expired" => {
                    let now = ctx["clockMonotonicMs"].as_u64().unwrap();
                    let expiry = ctx["secretExpiryMs"].as_u64().unwrap();
                    if now > expiry {
                        "reject"
                    } else {
                        "cache_replay"
                    }
                }
                "unauthenticated-media-request"
                    if ctx["mediaAuthorization"].as_str() == Some("absent") =>
                {
                    "reject"
                }
                "revoked-peer-control-connection" if ctx["peerRevoked"].as_bool() == Some(true) => {
                    "reject"
                }
                _ => "cache_replay",
            }
        }
        "delayed-event-after-stop" => {
            let payload = &messages[0]["payload"];
            let active = ctx["activeConnectionGeneration"].as_u64();
            let gen = payload["connectionGeneration"].as_u64();
            let state = payload["streamState"].as_str().unwrap();
            let after_stop = ctx["stateAfterStop"].as_str().unwrap();
            if active == gen && state != after_stop {
                // Same generation, but the event contradicts the acknowledged
                // stop projection: a delayed optimistic event is not state.
                "ignore"
            } else {
                "cache_replay"
            }
        }
        other => panic!("unknown semantic case: {}", other),
    }
}

#[test]
fn semantic_vectors_match_declared_outcomes() {
    let dir = fixtures_dir().join("semantic");
    let mut count = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let doc = load_json(&path);
        let case = doc.as_object().expect("semantic vector object");
        let declared = case["expect"].as_str().unwrap();
        let outcome = semantic_expect(case);
        assert_eq!(
            outcome,
            declared,
            "{}: Rust semantic outcome {} != declared {}",
            path.display(),
            outcome,
            declared
        );
        count += 1;
    }
    assert!(count >= 8, "expected >=8 semantic vectors, found {}", count);
}
