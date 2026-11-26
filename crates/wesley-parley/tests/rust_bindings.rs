//! The generated Rust module compiles, and real Parley lines round-trip through it. The lines are
//! captured from `bun run examples/count.ts files` in flyingrobots/parley's
//! `packages/parley-emit/README.md` (2026-11-22).

// The golden is prettyplease's output, compared byte for byte by `tests/rust.rs`; rustfmt must
// not reformat it.
#[rustfmt::skip]
#[path = "golden/messages.rs"]
#[allow(dead_code)]
mod messages;

use messages::{End, LensLossBound, ReceiptHead, ReceiptStep, RunBootstrap, SchemaRef};
use serde_json::{json, Value};

const BOOTSTRAP: &str = r#"{"event":"run","t":1795352899887,"seq":0,"run":"01M81NYC9E28S33150VJB8ZP7E","tool":"count","v":"0.1.0","inv":"638c446f866624d00aed41b516d92549f87b0197871ae5db312b7edaa8051a16","paths":{"receipt":"/private/tmp/parley-emit-readme.Ra5U/work/.parley/receipts/01M81NYC9E28S33150VJB8ZP7E.jsonl"},"_schema":{"id":"parley.run.bootstrap","version":"1.1.1"}}"#;
const HEAD: &str = r#"{"event":"receipt.head","t":1795352899887,"seq":1,"run":"01M81NYC9E28S33150VJB8ZP7E","tool":"count","v":"0.1.0","inv":"638c446f866624d00aed41b516d92549f87b0197871ae5db312b7edaa8051a16","argv":["count","files"],"resolved":{"command":"files","args":[]},"inputs":[],"effects":{"reads":["<cwd>/*"],"writes":[],"network":false,"idempotent":true,"bookkeeping":[{"op":"create","scope":"/private/tmp/parley-emit-readme.Ra5U/work/.parley/receipts/"},{"op":"create","scope":"/private/tmp/parley-emit-readme.Ra5U/work/.parley/.gitignore"},{"op":"delete","scope":"/private/tmp/parley-emit-readme.Ra5U/work/.parley/receipts/"},{"op":"create","scope":"/tmp/parley-emit-readme.Ra5U/state/parley/4b10e884505764fd/receipts/"},{"op":"delete","scope":"/tmp/parley-emit-readme.Ra5U/state/parley/4b10e884505764fd/receipts/"}]},"_schema":{"id":"parley.receipt.head","version":"2.0.0"}}"#;
const STEP: &str = r#"{"event":"receipt.step","t":1795352899890,"seq":2,"run":"01M81NYC9E28S33150VJB8ZP7E","decision":"counted 2 regular file(s)","because":"directories and special files are not files; only regular files count","detail":{"names":["b.txt","a.txt"]},"_schema":{"id":"parley.receipt.step","version":"1.0.0"}}"#;
const END: &str = r#"{"event":"end","t":1795352899890,"seq":4,"run":"01M81NYC9E28S33150VJB8ZP7E","status":0,"receipt":"/private/tmp/parley-emit-readme.Ra5U/work/.parley/receipts/01M81NYC9E28S33150VJB8ZP7E.jsonl","redacted":false,"_schema":{"id":"parley.end","version":"1.0.0"}}"#;

/// Deserialises a line into `T`, serialises it back, and requires the same JSON value.
fn round_trip<T: serde::de::DeserializeOwned + serde::Serialize>(line: &str) -> T {
    let parsed: T = serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}"));
    let back = serde_json::to_value(&parsed).expect("serialises");
    let original: Value = serde_json::from_str(line).expect("json");
    assert_eq!(back, original);
    parsed
}

#[test]
fn every_captured_line_round_trips_to_the_same_json_value() {
    let boot: RunBootstrap = round_trip(BOOTSTRAP);
    assert_eq!(boot.t, 1_795_352_899_887, "a 64-bit timestamp fits");
    assert_eq!(boot.session, None, "an absent optional field is None");
    let head: ReceiptHead = round_trip(HEAD);
    assert_eq!(head.argv, vec!["count".to_string(), "files".to_string()]);
    let _: ReceiptStep = round_trip(STEP);
    let end: End = round_trip(END);
    assert_eq!(end.status, 0);
}

#[test]
fn an_absent_optional_field_is_not_written_as_null() {
    let boot: RunBootstrap = serde_json::from_str(BOOTSTRAP).expect("parses");
    let written = serde_json::to_value(&boot).expect("serialises");
    assert!(
        written.get("session").is_none() && written.get("dry_run").is_none(),
        "{written}"
    );
}

#[test]
fn an_open_object_keeps_a_tools_own_fields() {
    let mut line: Value = serde_json::from_str(END).expect("json");
    line["tool_field"] = json!({"kept": [1, 2]});
    let end: End = serde_json::from_value(line.clone()).expect("parses");
    assert_eq!(serde_json::to_value(&end).expect("serialises"), line);
}

#[test]
fn the_closed_schema_reference_refuses_an_unknown_field() {
    let r: Result<SchemaRef, _> =
        serde_json::from_value(json!({"id": "parley.end", "version": "1.0.0", "extra": 1}));
    assert!(r.is_err());
}

#[test]
fn an_enum_reads_and_writes_its_wire_string() {
    let bound: LensLossBound = serde_json::from_value(json!("count-only")).expect("parses");
    assert_eq!(bound, LensLossBound::CountOnly);
    assert_eq!(
        serde_json::to_value(bound).expect("serialises"),
        json!("count-only")
    );
}

#[test]
fn each_message_names_its_schema() {
    assert_eq!(
        (End::SCHEMA_ID, End::SCHEMA_VERSION),
        ("parley.end", "1.0.0")
    );
}

#[test]
fn an_explicit_null_in_an_optional_json_field_survives_a_round_trip() {
    let line = r#"{"event":"error","t":1,"seq":3,"run":"r","code":"x.y","problem":"p","measured":null,"_schema":{"id":"parley.error","version":"2.0.0"}}"#;
    let parsed: messages::Error = serde_json::from_str(line).expect("parses");
    assert_eq!(parsed.measured, Some(Value::Null), "a present null is kept");
    assert_eq!(
        serde_json::to_value(&parsed).expect("serialises"),
        serde_json::from_str::<Value>(line).expect("json")
    );
    let absent: messages::Error =
        serde_json::from_str(&line.replace(r#""measured":null,"#, "")).expect("parses");
    assert_eq!(absent.measured, None, "an absent field is None");
}
