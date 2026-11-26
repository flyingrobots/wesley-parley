//! Parley's real SDL, lowered by the published Wesley, reproduces each of Parley's 17
//! hand-written message schemas as a JSON value (rule (A) of flyingrobots/parley#116).

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;
use wesley_parley::{catalog, json_schema};

fn fixture(path: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/parley")
        .join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn hand_written() -> BTreeMap<String, Value> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parley/schemas");
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(dir).expect("schemas dir") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("name")
            .to_string();
        let id = name
            .strip_suffix(".schema.json")
            .expect("schema file")
            .to_string();
        let text = std::fs::read_to_string(&path).expect("read");
        out.insert(id, serde_json::from_str(&text).expect("json"));
    }
    out
}

fn generated() -> BTreeMap<String, Value> {
    let ir =
        wesley_core::lower_schema_sdl(&fixture("parley.graphql")).expect("Parley's SDL lowers");
    let catalog = catalog(&ir).expect("Parley's SDL is valid Parley vocabulary");
    json_schema::documents(&catalog)
        .into_iter()
        .map(|d| (d.id, d.value))
        .collect()
}

/// Every difference between two JSON values, as `pointer: generated vs hand-written`.
fn differences(at: &str, a: &Value, b: &Value, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let p = format!("{at}/{}", k.replace('~', "~0").replace('/', "~1"));
                match (x.get(k), y.get(k)) {
                    (Some(xa), Some(yb)) => differences(&p, xa, yb, out),
                    (xa, yb) => out.push(format!("{p}: generated {xa:?}, hand-written {yb:?}")),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (xa, yb)) in x.iter().zip(y).enumerate() {
                differences(&format!("{at}/{i}"), xa, yb, out);
            }
        }
        _ if a == b => {}
        _ => out.push(format!("{at}: generated {a}, hand-written {b}")),
    }
}

#[test]
fn the_sdl_declares_exactly_the_messages_parley_has() {
    let ids: Vec<String> = generated().into_keys().collect();
    let expected: Vec<String> = hand_written().into_keys().collect();
    assert_eq!(ids, expected);
    assert_eq!(ids.len(), 17);
}

#[test]
fn every_generated_schema_equals_the_hand_written_one_as_a_json_value() {
    let generated = generated();
    let mut all = Vec::new();
    for (id, schema) in hand_written() {
        let Some(g) = generated.get(&id) else {
            all.push(format!("{id}: not generated"));
            continue;
        };
        let mut diffs = Vec::new();
        differences("", g, &schema, &mut diffs);
        all.extend(diffs.into_iter().map(|d| format!("{id}: {d}")));
    }
    assert!(
        all.is_empty(),
        "{} differences:\n{}",
        all.len(),
        all.join("\n")
    );
}

#[test]
fn a_change_in_the_sdl_alone_is_seen() {
    let sdl = fixture("parley.graphql").replacen(
        "run: String! @parley_minLength(value: 1)\n  \"Working period",
        "run: String! @parley_minLength(value: 2)\n  \"Working period",
        1,
    );
    assert_ne!(sdl, fixture("parley.graphql"), "the edit applies");
    let ir = wesley_core::lower_schema_sdl(&sdl).expect("lowers");
    let docs = json_schema::documents(&catalog(&ir).expect("valid"));
    let bootstrap = docs
        .iter()
        .find(|d| d.id == "parley.run.bootstrap")
        .expect("bootstrap");
    assert_eq!(bootstrap.value["properties"]["run"]["minLength"], 2);
}

#[test]
fn documents_are_named_by_message_id_and_printed_canonically() {
    let ir = wesley_core::lower_schema_sdl(&fixture("parley.graphql")).expect("lowers");
    let docs = json_schema::documents(&catalog(&ir).expect("valid"));
    let end = docs.iter().find(|d| d.id == "parley.end").expect("end");
    assert_eq!(end.file_name, "parley.end.schema.json");
    let printed = json_schema::print(end);
    assert!(printed.starts_with("{\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n  \"$id\": \"https://flyingrobots.dev/parley/schemas/parley.end/1.0.0\",\n"));
    assert!(printed.ends_with("}\n"));
    assert!(printed.contains("§3.3"), "non-ASCII written as itself");
}
