//! `wesley-parley`: reads the Wesley IR of a Parley SDL and writes Parley's artefacts.
//!
//! ```text
//! wesley schema lower --schema schemas/parley.graphql --json | wesley-parley json-schema --out schemas/
//! wesley schema lower --schema schemas/parley.graphql --json | wesley-parley model
//! ```

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage:\n  wesley-parley json-schema --out <dir> [--ir <file>]   write <id>.schema.json for every message\n  wesley-parley typescript --out <file> [--ir <file>]   write the message types as one TypeScript module\n  wesley-parley rust --out <file> [--ir <file>]         write the message types as one Rust module\n  wesley-parley model [--ir <file>]                     print the interpreted model as JSON\n\nThe IR is Wesley's (`wesley schema lower --json`), read from --ir or stdin.";

enum Command {
    JsonSchema { out: PathBuf },
    TypeScript { out: PathBuf },
    Rust { out: PathBuf },
    Model,
}

fn parse(mut args: impl Iterator<Item = String>) -> Result<(Command, Option<PathBuf>), String> {
    let command = args.next();
    let (mut out, mut ir) = (None, None);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--out" => out = Some(PathBuf::from(value()?)),
            "--ir" => ir = Some(PathBuf::from(value()?)),
            other => return Err(format!("unknown option {other}")),
        }
    }
    let command = match command.as_deref() {
        Some("json-schema") => Command::JsonSchema {
            out: out.ok_or("--out is required")?,
        },
        Some("typescript") => Command::TypeScript {
            out: out.ok_or("--out is required")?,
        },
        Some("rust") => Command::Rust {
            out: out.ok_or("--out is required")?,
        },
        Some("model") if out.is_none() => Command::Model,
        Some("model") => return Err("model prints to stdout; it takes no --out".into()),
        Some("--help" | "-h") | None => return Err(String::new()),
        Some(other) => return Err(format!("unknown command {other}")),
    };
    Ok((command, ir))
}

fn run(command: &Command, ir: Option<&PathBuf>) -> Result<(), String> {
    let text = match ir {
        Some(path) => {
            std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?
        }
        None => {
            let mut s = String::new();
            std::io::stdin()
                .read_to_string(&mut s)
                .map_err(|e| format!("stdin: {e}"))?;
            s
        }
    };
    let ir = serde_json::from_str(&text).map_err(|e| format!("IR: {e}"))?;
    let catalog = wesley_parley::catalog(&ir).map_err(|e| e.to_string())?;
    match command {
        Command::JsonSchema { out } => {
            std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
            for d in wesley_parley::json_schema::documents(&catalog) {
                let path = out.join(&d.file_name);
                std::fs::write(&path, wesley_parley::json_schema::print(&d))
                    .map_err(|e| format!("{}: {e}", path.display()))?;
            }
        }
        Command::TypeScript { out } => {
            let module = wesley_parley::typescript::render(&catalog).map_err(|e| e.to_string())?;
            std::fs::write(out, module).map_err(|e| format!("{}: {e}", out.display()))?;
        }
        Command::Rust { out } => {
            let module = wesley_parley::rust::render(&catalog).map_err(|e| e.to_string())?;
            std::fs::write(out, module).map_err(|e| format!("{}: {e}", out.display()))?;
        }
        Command::Model => {
            let value = serde_json::to_value(&catalog).map_err(|e| e.to_string())?;
            std::io::stdout()
                .write_all(wesley_json_schema::to_document_string(&value).as_bytes())
                .map_err(|e| format!("stdout: {e}"))?;
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let (command, ir) = match parse(std::env::args().skip(1)) {
        Ok(parsed) => parsed,
        Err(message) if message.is_empty() => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("wesley-parley: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&command, ir.as_ref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("wesley-parley: {message}");
            ExitCode::FAILURE
        }
    }
}
