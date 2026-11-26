//! Parley's per-message versioning (spec §4.5; flyingrobots/parley#122), checked against the
//! released snapshots (`schemas/released/<id>/<version>.schema.json`).
//!
//! - A message at a released version must render to that snapshot's JSON value (rule (A) of
//!   flyingrobots/parley#116): a released schema never changes.
//! - A message at an unreleased version is a draft. It may change in place, but its version must be
//!   at least as big a bump over the latest release as its change needs: a description edit needs a
//!   patch, a new optional field a minor, and anything else (a new required field, a changed
//!   constraint, a removed field) a major.

use std::cmp::Ordering;
use std::path::Path;

use serde_json::{Map, Value};

use crate::json_schema::documents;
use crate::model::Catalog;
use crate::Error;

/// How big a version change is, or needs to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bump {
    /// No change to the schema beyond its own identity.
    None,
    /// Descriptions only.
    Patch,
    /// Only new optional fields (and descriptions).
    Minor,
    /// Anything else.
    Major,
}

/// The verdict on one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// At a released version, and identical to its snapshot.
    Released,
    /// No version of this message has been released.
    New,
    /// An unreleased version, bumped enough over the latest release `from`.
    Draft {
        /// The latest released version.
        from: String,
        /// The bump its change needs.
        needed: Bump,
    },
    /// At a released version, but its schema differs from the snapshot.
    ChangedAfterRelease,
    /// An unreleased version whose bump over `from` is smaller than its change needs.
    BumpTooSmall {
        /// The latest released version.
        from: String,
        /// The bump the change needs.
        needed: Bump,
        /// The bump the version got.
        given: Bump,
    },
    /// An unreleased version lower than the latest release.
    BelowLatestRelease {
        /// The latest released version.
        latest: String,
    },
}

impl Verdict {
    /// Whether the verdict lets a release proceed.
    #[must_use]
    pub const fn is_ok(&self) -> bool {
        matches!(self, Self::Released | Self::New | Self::Draft { .. })
    }
}

type Version = (u64, u64, u64);

fn parse_version(s: &str) -> Option<Version> {
    let mut parts = s.split('.').map(|p| p.parse::<u64>().ok());
    let v = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(v)
}

/// Checks every message of the catalog against the released snapshots under `released`.
///
/// # Errors
///
/// [`Error::Released`] when a snapshot cannot be read or a version is not `MAJOR.MINOR.PATCH`.
pub fn check(catalog: &Catalog, released: &Path) -> Result<Vec<(String, Verdict)>, Error> {
    // A missing or unreadable snapshot root is an error, never an empty history: otherwise a
    // misspelt path would pass every changed release as new.
    if !released.is_dir() {
        return Err(Error::Released(format!(
            "{} is not a directory of released snapshots",
            released.display()
        )));
    }
    let mut out = Vec::new();
    for doc in documents(catalog) {
        let current = parse_version(&doc.version).ok_or_else(|| {
            Error::Released(format!(
                "{}: version {} is not MAJOR.MINOR.PATCH",
                doc.id, doc.version
            ))
        })?;
        let versions = released_versions(&released.join(&doc.id))?;
        let read = |v: &str| -> Result<Value, Error> {
            let path = released.join(&doc.id).join(format!("{v}.schema.json"));
            let text = std::fs::read_to_string(&path)
                .map_err(|e| Error::Released(format!("{}: {e}", path.display())))?;
            serde_json::from_str(&text)
                .map_err(|e| Error::Released(format!("{}: {e}", path.display())))
        };
        let verdict = if versions.iter().any(|(_, s)| *s == doc.version) {
            if read(&doc.version)? == doc.value {
                Verdict::Released
            } else {
                Verdict::ChangedAfterRelease
            }
        } else if let Some((latest, latest_text)) = versions.last() {
            if current.cmp(latest) == Ordering::Less {
                Verdict::BelowLatestRelease {
                    latest: latest_text.clone(),
                }
            } else {
                let needed = needed_bump(&read(latest_text)?, &doc.value);
                let given = given_bump(*latest, current);
                if given < needed {
                    Verdict::BumpTooSmall {
                        from: latest_text.clone(),
                        needed,
                        given,
                    }
                } else {
                    Verdict::Draft {
                        from: latest_text.clone(),
                        needed,
                    }
                }
            }
        } else {
            Verdict::New
        };
        out.push((doc.id, verdict));
    }
    Ok(out)
}

/// The released versions of one message, lowest first.
fn released_versions(dir: &Path) -> Result<Vec<(Version, String)>, Error> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        // No directory for this message under a valid root: it has never been released.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::Released(format!("{}: {e}", dir.display()))),
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| Error::Released(format!("{}: {e}", dir.display())))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(text) = name.strip_suffix(".schema.json") else {
            continue;
        };
        let v = parse_version(text).ok_or_else(|| {
            Error::Released(format!(
                "{}: {name} is not <MAJOR.MINOR.PATCH>.schema.json",
                dir.display()
            ))
        })?;
        out.push((v, text.to_string()));
    }
    out.sort();
    Ok(out)
}

fn given_bump(from: Version, to: Version) -> Bump {
    if to.0 != from.0 {
        Bump::Major
    } else if to.1 != from.1 {
        Bump::Minor
    } else if to.2 != from.2 {
        Bump::Patch
    } else {
        Bump::None
    }
}

/// The bump a change from the `old` schema to the `new` one needs (both whole message schemas).
pub fn needed_bump(old: &Value, new: &Value) -> Bump {
    let (old, new) = (without_identity(old), without_identity(new));
    if old == new {
        return Bump::None;
    }
    let (old_bare, new_bare) = (without_descriptions(&old), without_descriptions(&new));
    if old_bare == new_bare {
        return Bump::Patch;
    }
    if without_new_optional_fields(&old_bare, &new_bare) == old_bare {
        return Bump::Minor;
    }
    Bump::Major
}

/// A schema without what changes with every version: its `$id`, and the pinned `_schema.version`.
fn without_identity(schema: &Value) -> Value {
    let mut s = schema.clone();
    if let Value::Object(map) = &mut s {
        map.remove("$id");
        if let Some(Value::Object(version)) = map
            .get_mut("properties")
            .and_then(|p| p.get_mut("_schema"))
            .and_then(|r| r.get_mut("properties"))
            .and_then(|p| p.get_mut("version"))
        {
            version.remove("const");
        }
    }
    s
}

/// Keywords whose value is one subschema.
const ONE_SUBSCHEMA: &[&str] = &["items", "not", "if", "then", "else"];
/// Keywords whose value is a list of subschemas.
const SUBSCHEMA_LISTS: &[&str] = &["anyOf", "allOf", "oneOf"];

/// A schema without its `description` annotations. Only schema nodes are walked: a property
/// *named* `description` is a field, and a `const` or `enum` value is data, so neither is touched.
fn without_descriptions(schema: &Value) -> Value {
    let Value::Object(map) = schema else {
        return schema.clone();
    };
    let mut out = Map::new();
    for (k, v) in map {
        let v = match (k.as_str(), v) {
            ("description", _) => continue,
            ("properties", Value::Object(props)) => Value::Object(
                props
                    .iter()
                    .map(|(name, s)| (name.clone(), without_descriptions(s)))
                    .collect(),
            ),
            (k, Value::Array(list)) if SUBSCHEMA_LISTS.contains(&k) => {
                Value::Array(list.iter().map(without_descriptions).collect())
            }
            (k, s) if ONE_SUBSCHEMA.contains(&k) => without_descriptions(s),
            (_, other) => other.clone(),
        };
        out.insert(k.clone(), v);
    }
    Value::Object(out)
}

/// `new` with every optional property it adds, at any depth, taken out: the properties of an
/// object node that the matching `old` node lacks and the new node does not require. The two
/// schemas are walked together through `properties`, single subschemas and subschema lists.
fn without_new_optional_fields(old: &Value, new: &Value) -> Value {
    let (Value::Object(old_map), Value::Object(new_map)) = (old, new) else {
        return new.clone();
    };
    let required: Vec<&Value> = new_map
        .get("required")
        .and_then(Value::as_array)
        .map(|r| r.iter().collect())
        .unwrap_or_default();
    let mut out = Map::new();
    for (k, v) in new_map {
        let v = match (k.as_str(), v, old_map.get(k)) {
            ("properties", Value::Object(new_props), Some(Value::Object(old_props))) => {
                Value::Object(
                    new_props
                        .iter()
                        .filter(|(name, _)| {
                            old_props.contains_key(*name)
                                || required.contains(&&Value::String((*name).clone()))
                        })
                        .map(|(name, s)| {
                            (
                                name.clone(),
                                old_props.get(name).map_or_else(
                                    || s.clone(),
                                    |o| without_new_optional_fields(o, s),
                                ),
                            )
                        })
                        .collect(),
                )
            }
            (k, Value::Array(list), Some(Value::Array(old_list)))
                if SUBSCHEMA_LISTS.contains(&k) && list.len() == old_list.len() =>
            {
                Value::Array(
                    old_list
                        .iter()
                        .zip(list)
                        .map(|(o, n)| without_new_optional_fields(o, n))
                        .collect(),
                )
            }
            (k, s, Some(o)) if ONE_SUBSCHEMA.contains(&k) => without_new_optional_fields(o, s),
            (_, other, _) => other.clone(),
        };
        out.insert(k.clone(), v);
    }
    Value::Object(out)
}
