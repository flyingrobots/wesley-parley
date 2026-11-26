//! The `wesley-parley` binary, end to end on Parley's real SDL.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn ir_text() -> String {
    let sdl = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parley/parley.graphql"),
    )
    .expect("sdl");
    serde_json::to_string(&wesley_core::lower_schema_sdl(&sdl).expect("lowers")).expect("ir")
}

fn run(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wesley-parley"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("writes");
    child.wait_with_output().expect("exits")
}

#[test]
fn json_schema_writes_one_file_per_message_equal_to_parleys_as_json() {
    let out = std::env::temp_dir().join(format!("wesley-parley-cli-{}", std::process::id()));
    let o = run(
        &["json-schema", "--out", out.to_str().expect("utf8")],
        &ir_text(),
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parley/schemas");
    let mut count = 0;
    for entry in std::fs::read_dir(&fixtures).expect("fixtures") {
        let name = entry.expect("entry").file_name();
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out.join(&name)).expect("written"))
                .expect("json");
        let expected: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(fixtures.join(&name)).expect("fixture"))
                .expect("json");
        assert_eq!(written, expected, "{name:?}");
        count += 1;
    }
    assert_eq!(count, 17);
    std::fs::remove_dir_all(&out).expect("cleaned");
}

#[test]
fn model_prints_the_interpreted_catalog() {
    let o = run(&["model"], &ir_text());
    assert!(o.status.success());
    let model: serde_json::Value = serde_json::from_slice(&o.stdout).expect("json");
    assert_eq!(model["messages"].as_array().map(Vec::len), Some(17));
    assert_eq!(
        model["objects"]["End"]["fields"][0]["value"],
        serde_json::json!({"kind": "const", "value": "end"})
    );
}

#[test]
fn a_vocabulary_error_exits_1_and_names_the_rule() {
    let o = run(
        &["model"],
        r#"{"version":"1.0.0","types":[{"name":"A","kind":"OBJECT","directives":{"parley_bogus":true},"fields":[]}]}"#,
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("unknown directive @parley_bogus"));
}

#[test]
fn a_missing_out_is_a_usage_error() {
    assert_eq!(run(&["json-schema"], "").status.code(), Some(2));
}
