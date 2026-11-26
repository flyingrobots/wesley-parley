//! A Wesley target for Parley.
//!
//! Wesley compiles GraphQL SDL to an IR and attaches no meaning to directives. Parley declares its
//! own vocabulary, the `@parley_*` directives, in its own SDL; this crate is the one place that
//! vocabulary means something. [`catalog`] interprets it into a [`model::Catalog`], and each
//! renderer reads only that model.

#![deny(missing_docs)]

pub mod json_schema;
pub mod model;
pub mod vocab;

pub use vocab::catalog;

/// Why a Parley SDL could not be interpreted.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The SDL breaks a rule of Parley's vocabulary.
    #[error("Parley vocabulary: {0}")]
    Vocabulary(String),
}
