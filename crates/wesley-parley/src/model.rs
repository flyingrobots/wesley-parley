//! The meaning of a Parley SDL, after its `@parley_*` directives are interpreted and checked.
//!
//! Every renderer (JSON Schema, TypeScript, Rust, Python) reads this model and nothing else, so
//! Parley's vocabulary is interpreted exactly once. The model serialises to JSON for renderers
//! that run in another language (the Python bindings).

use indexmap::IndexMap;
use serde::Serialize;

/// Everything a Parley SDL declares.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Catalog {
    /// The message types, in the SDL's type-name order.
    pub messages: Vec<Message>,
    /// Every object type a message reaches, keyed by type name, in type-name order.
    pub objects: IndexMap<String, Object>,
}

/// A message: an object type with a schema identity.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Message {
    /// The message id, e.g. `parley.end`.
    pub id: String,
    /// The message's schema version.
    pub version: String,
    /// The schema title.
    pub title: String,
    /// The object type that holds the message's fields.
    pub object: String,
}

/// An object type.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Object {
    /// The GraphQL type name.
    pub name: String,
    /// The type's description. Only a message type may have one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the object admits fields it does not name.
    pub open: bool,
    /// The fields, in declaration order.
    pub fields: Vec<Field>,
    /// Rules across fields.
    pub rules: Rules,
}

/// A field of an object.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Field {
    /// The field's wire name.
    pub name: String,
    /// The field's description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Required, or optional (absent when unset, never null).
    pub required: bool,
    /// What the field holds.
    pub value: Value,
}

/// What a field (or a list's item) holds.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Value {
    /// A JSON string.
    String(StringConstraints),
    /// A JSON integer.
    Integer {
        /// 32 for GraphQL `Int`, 64 for a 64-bit scalar.
        bits: u8,
        /// Value constraints.
        constraints: IntegerConstraints,
    },
    /// A JSON boolean.
    Boolean,
    /// Any JSON value.
    Any,
    /// Any JSON object.
    JsonObject {
        /// The fewest members the object may have.
        #[serde(skip_serializing_if = "Option::is_none")]
        min_properties: Option<u64>,
    },
    /// Exactly this literal value.
    Const {
        /// The literal.
        value: serde_json::Value,
    },
    /// One of these strings.
    StringEnum {
        /// The allowed strings, in order.
        values: Vec<String>,
        /// The GraphQL enum type that declares them, when they come from one.
        #[serde(skip_serializing_if = "Option::is_none")]
        type_name: Option<String>,
    },
    /// The enclosing message's own identity: a closed `{id, version}` object.
    MessageRef {
        /// The object type that shapes it.
        object: String,
        /// The message id it is pinned to.
        id: String,
        /// The message version it is pinned to.
        version: String,
    },
    /// A nested object.
    Object {
        /// The object type.
        name: String,
        /// The fewest members the object may have.
        #[serde(skip_serializing_if = "Option::is_none")]
        min_properties: Option<u64>,
    },
    /// A list.
    List {
        /// What each item holds.
        item: Box<Value>,
        /// The fewest items.
        #[serde(skip_serializing_if = "Option::is_none")]
        min_items: Option<u64>,
        /// The most items.
        #[serde(skip_serializing_if = "Option::is_none")]
        max_items: Option<u64>,
    },
}

/// Constraints on a string.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StringConstraints {
    /// An ECMA-262 regular expression the string matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// The fewest characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u64>,
    /// A JSON Schema format name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Constraints on an integer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegerConstraints {
    /// The least value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,
    /// The greatest value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,
    /// A value the integer never takes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_equal: Option<i64>,
}

/// Rules across an object's fields.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rules {
    /// At most one of these two fields is present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive: Option<[String; 2]>,
    /// At least one of these list fields is present and non-empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_non_empty: Option<Vec<String>>,
    /// A conditional rule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,
}

/// When the field at `path` equals `equals`, the named fields of its parent are required, or hold
/// the given values; with `forbid_otherwise`, the one required field is forbidden when it does not.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct When {
    /// Field names from the object to the trigger; every step but the last is an object field.
    pub path: Vec<String>,
    /// The trigger value.
    pub equals: serde_json::Value,
    /// Whether the rule fires only when every field on the path is present.
    pub guarded: bool,
    /// Fields of the trigger's parent that become required.
    pub require: Vec<String>,
    /// Whether the required field is forbidden when the trigger does not hold.
    pub forbid_otherwise: bool,
    /// Fields of the trigger's parent that must hold these values.
    pub set: IndexMap<String, serde_json::Value>,
}
