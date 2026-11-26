//! Parley's per-message versioning (spec §4.5, flyingrobots/parley#122): a released version's
//! schema never changes, and a new version is at least as big a bump as its change needs.

use std::path::{Path, PathBuf};

use wesley_parley::versions::{check, Bump, Verdict};

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn released() -> PathBuf {
    manifest().join("tests/fixtures/parley/released")
}

fn sdl() -> String {
    std::fs::read_to_string(manifest().join("tests/fixtures/parley/parley.graphql")).expect("sdl")
}

fn verdicts(sdl: &str) -> Vec<(String, Verdict)> {
    let ir = wesley_core::lower_schema_sdl(sdl).expect("lowers");
    let catalog = wesley_parley::catalog(&ir).expect("valid");
    check(&catalog, &released()).expect("reads the snapshots")
}

fn verdict(sdl: &str, id: &str) -> Verdict {
    verdicts(sdl)
        .into_iter()
        .find(|(i, _)| i == id)
        .map(|(_, v)| v)
        .expect("message")
}

/// Edits the `parley.end` message: its version and one extra line in its body.
fn edit_end(version: &str, extra_field: &str) -> String {
    let s = sdl().replacen(
        "@parley_message(id: \"parley.end\", version: \"1.0.0\"",
        &format!("@parley_message(id: \"parley.end\", version: \"{version}\""),
        1,
    );
    let anchor = "(spec §3.3).\"\n  redacted: Boolean!\n";
    assert_eq!(s.matches(anchor).count(), 1, "the anchor is End's alone");
    s.replacen(anchor, &format!("{anchor}{extra_field}"), 1)
}

#[test]
fn parleys_sdl_at_c80771c_is_every_message_at_its_released_version() {
    let all = verdicts(&sdl());
    assert_eq!(all.len(), 17);
    for (id, v) in all {
        assert_eq!(v, Verdict::Released, "{id}");
    }
}

#[test]
fn the_edit_helper_applies() {
    assert_ne!(edit_end("1.0.0", "  x: String @parley_optional\n"), sdl());
}

#[test]
fn a_released_version_whose_schema_changed_is_refused() {
    let v = verdict(
        &edit_end("1.0.0", "  \"added\"\n  x: String @parley_optional\n"),
        "parley.end",
    );
    assert_eq!(v, Verdict::ChangedAfterRelease);
}

#[test]
fn a_new_required_field_needs_a_major_bump() {
    let v = verdict(&edit_end("1.1.0", "  x: String!\n"), "parley.end");
    assert_eq!(
        v,
        Verdict::BumpTooSmall {
            from: "1.0.0".into(),
            needed: Bump::Major,
            given: Bump::Minor
        }
    );
    assert_eq!(
        verdict(&edit_end("2.0.0", "  x: String!\n"), "parley.end"),
        Verdict::Draft {
            from: "1.0.0".into(),
            needed: Bump::Major
        }
    );
}

#[test]
fn a_new_optional_field_needs_a_minor_bump() {
    let field = "  x: String @parley_optional\n";
    assert_eq!(
        verdict(&edit_end("1.0.1", field), "parley.end"),
        Verdict::BumpTooSmall {
            from: "1.0.0".into(),
            needed: Bump::Minor,
            given: Bump::Patch
        }
    );
    assert_eq!(
        verdict(&edit_end("1.1.0", field), "parley.end"),
        Verdict::Draft {
            from: "1.0.0".into(),
            needed: Bump::Minor
        }
    );
}

#[test]
fn a_description_edit_needs_a_patch() {
    let s = sdl().replacen(
        "\"The exit status the process is about to exit with (spec §3.4); never 3,",
        "\"The status the process is about to exit with (spec §3.4); never 3,",
        1,
    );
    assert_ne!(s, sdl(), "the edit applies");
    assert_eq!(verdict(&s, "parley.end"), Verdict::ChangedAfterRelease);
    let s = s.replacen(
        "id: \"parley.end\", version: \"1.0.0\"",
        "id: \"parley.end\", version: \"1.0.1\"",
        1,
    );
    assert_eq!(
        verdict(&s, "parley.end"),
        Verdict::Draft {
            from: "1.0.0".into(),
            needed: Bump::Patch
        }
    );
}

#[test]
fn a_version_below_the_latest_release_is_refused() {
    let s = sdl().replacen(
        "id: \"parley.query\", version: \"1.1.0\"",
        "id: \"parley.query\", version: \"1.0.5\"",
        1,
    );
    assert_eq!(
        verdict(&s, "parley.query"),
        Verdict::BelowLatestRelease {
            latest: "1.1.0".into()
        }
    );
}

#[test]
fn a_message_never_released_is_new() {
    let s = format!(
        "{}\n\"A brand new message.\"\ntype Fresh @parley_message(id: \"parley.fresh\", version: \"0.1.0\", title: \"parley fresh\") @parley_open {{\n  event: String! @parley_const(string: \"fresh\")\n  _schema: SchemaRef! @parley_message_ref\n}}\n",
        sdl()
    );
    assert_eq!(verdict(&s, "parley.fresh"), Verdict::New);
}

fn snapshot(id: &str, version: &str) -> serde_json::Value {
    let path = released().join(id).join(format!("{version}.schema.json"));
    serde_json::from_str(&std::fs::read_to_string(path).expect("snapshot")).expect("json")
}

#[test]
fn parleys_real_lens_1_1_0_release_is_a_minor_change_over_1_0_0() {
    use wesley_parley::versions::needed_bump;
    assert_eq!(
        needed_bump(
            &snapshot("parley.lens", "1.0.0"),
            &snapshot("parley.lens", "1.1.0")
        ),
        Bump::Minor
    );
    assert_eq!(
        needed_bump(
            &snapshot("parley.query", "1.0.0"),
            &snapshot("parley.query", "1.1.0")
        ),
        Bump::Minor
    );
}

#[test]
fn a_field_named_description_is_a_field_not_an_annotation() {
    use wesley_parley::versions::needed_bump;
    let old =
        serde_json::json!({"type": "object", "properties": {"description": {"type": "string"}}});
    let new =
        serde_json::json!({"type": "object", "properties": {"description": {"type": "integer"}}});
    assert_eq!(needed_bump(&old, &new), Bump::Major);
    let described = serde_json::json!({"type": "object", "properties": {"description": {"type": "string", "description": "words"}}});
    assert_eq!(needed_bump(&old, &described), Bump::Patch);
}

#[test]
fn a_released_directory_that_does_not_exist_is_an_error_not_an_empty_history() {
    let ir = wesley_core::lower_schema_sdl(&sdl()).expect("lowers");
    let catalog = wesley_parley::catalog(&ir).expect("valid");
    let r = wesley_parley::versions::check(
        &catalog,
        &manifest().join("tests/fixtures/parley/no-such-dir"),
    );
    assert!(r.is_err());
    let r = wesley_parley::versions::check(
        &catalog,
        &manifest().join("tests/fixtures/parley/parley.graphql"),
    );
    assert!(r.is_err(), "a file is not a snapshot directory");
}
