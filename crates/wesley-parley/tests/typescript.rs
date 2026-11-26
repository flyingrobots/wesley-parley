//! Parley's message types in TypeScript (flyingrobots/parley#120), rendered from the model through
//! wesley-typescript's swc syntax tree, and type-checked by the real TypeScript compiler.

use std::path::{Path, PathBuf};
use std::process::Command;

use wesley_parley::{catalog, typescript};

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn generated() -> String {
    let sdl = std::fs::read_to_string(manifest().join("tests/fixtures/parley/parley.graphql"))
        .expect("sdl");
    let ir = wesley_core::lower_schema_sdl(&sdl).expect("lowers");
    typescript::render(&catalog(&ir).expect("valid")).expect("renders")
}

#[test]
fn the_module_matches_its_golden_file() {
    let path = manifest().join("tests/golden/messages.generated.ts");
    let printed = generated();
    if std::env::var_os("WESLEY_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &printed).expect("golden written");
    }
    assert_eq!(printed, std::fs::read_to_string(&path).expect("golden"));
}

#[test]
fn rendering_is_deterministic() {
    assert_eq!(generated(), generated());
}

#[test]
fn the_shapes_parley_needs_are_there() {
    let ts = generated();
    for needle in [
        "export interface End {",
        "    event: \"end\";",
        "    _schema: {\n        id: \"parley.end\";\n        /** Full validation is against the schema for this exact id and version (spec §4.5, §4.6). */\n        version: \"1.0.0\";\n    };",
        "    dry_run?: boolean;",
        "export type Line = ",
        "export interface InputDescriptor {",
    ] {
        assert!(ts.contains(needle), "missing {needle:?}");
    }
}

/// Runs `tsc --noEmit --strict` over the given files; `None` when no TypeScript toolchain is
/// available off CI.
fn tsc(files: &[PathBuf]) -> Option<std::process::Output> {
    let has_npx = Command::new("npx")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !has_npx {
        assert!(
            std::env::var_os("CI").is_none(),
            "CI must type-check the generated TypeScript; npx is missing"
        );
        eprintln!("skipping the tsc check: npx is not on PATH");
        return None;
    }
    let mut cmd = Command::new("npx");
    cmd.args([
        "--yes",
        "-p",
        "typescript@5",
        "tsc",
        "--noEmit",
        "--strict",
        "--target",
        "es2022",
        "--module",
        "es2022",
        "--moduleResolution",
        "bundler",
    ]);
    cmd.args(files);
    Some(cmd.output().expect("tsc runs"))
}

#[test]
fn the_golden_module_and_its_valid_uses_type_check() {
    let files = [
        manifest().join("tests/golden/messages.generated.ts"),
        manifest().join("tests/typescript/pass/uses.ts"),
    ];
    if let Some(o) = tsc(&files) {
        assert!(
            o.status.success(),
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

#[test]
fn each_invalid_use_is_a_type_error() {
    for (fixture, code) in [
        ("missing_required.ts", "TS2741"),
        ("wrong_event.ts", "TS2322"),
        ("null_optional.ts", "TS2322"),
    ] {
        let files = [
            manifest().join("tests/golden/messages.generated.ts"),
            manifest().join("tests/typescript/fail").join(fixture),
        ];
        if let Some(o) = tsc(&files) {
            let out = String::from_utf8_lossy(&o.stdout);
            assert!(
                !o.status.success() && out.contains(code),
                "{fixture}: expected {code}, got:\n{out}"
            );
        }
    }
}
