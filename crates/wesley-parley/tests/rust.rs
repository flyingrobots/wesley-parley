//! The generated Rust module for Parley's real SDL is fresh. `tests/rust_bindings.rs` compiles it.

use std::path::Path;

fn generated() -> String {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sdl = std::fs::read_to_string(manifest.join("tests/fixtures/parley/parley.graphql"))
        .expect("sdl");
    let ir = wesley_core::lower_schema_sdl(&sdl).expect("lowers");
    wesley_parley::rust::render(&wesley_parley::catalog(&ir).expect("valid")).expect("renders")
}

#[test]
fn the_module_matches_its_golden_file() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/messages.rs");
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
