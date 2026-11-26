//! The interpreted model of Parley's SDL, as the JSON `wesley-parley model` prints. Renderers in
//! other languages (the Python bindings under `python/`) read this golden, so it must stay fresh.

use std::path::Path;

#[test]
fn the_model_of_parleys_sdl_matches_its_golden_file() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sdl = std::fs::read_to_string(manifest.join("tests/fixtures/parley/parley.graphql"))
        .expect("sdl");
    let ir = wesley_core::lower_schema_sdl(&sdl).expect("lowers");
    let catalog = wesley_parley::catalog(&ir).expect("valid");
    let printed = wesley_json_schema::to_document_string(
        &serde_json::to_value(&catalog).expect("serialises"),
    );
    let path = manifest.join("tests/golden/model.json");
    if std::env::var_os("WESLEY_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &printed).expect("golden written");
    }
    assert_eq!(printed, std::fs::read_to_string(&path).expect("golden"));
}
