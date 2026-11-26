//! Parley's message schemas, rendered from the [`model`](crate::model) as JSON Schema 2020-12
//! values and printed by `wesley-json-schema`'s canonical printer.

use serde_json::{json, Map, Value as Json};

use crate::model::{Catalog, Field, Message, Object, Value, When};
use crate::vocab::json_type;

/// Where Parley's message schemas live: a message's `$id` is this, its id, `/`, and its version.
pub const ID_BASE: &str = "https://flyingrobots.dev/parley/schemas/";

/// One message's schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The message id.
    pub id: String,
    /// The message version.
    pub version: String,
    /// `<id>.schema.json`, the name Parley gives the file.
    pub file_name: String,
    /// The schema.
    pub value: Json,
}

/// Every message's schema, in the catalog's message order.
#[must_use]
pub fn documents(catalog: &Catalog) -> Vec<Document> {
    catalog
        .messages
        .iter()
        .map(|m| Document {
            id: m.id.clone(),
            version: m.version.clone(),
            file_name: format!("{}.schema.json", m.id),
            value: document(catalog, m),
        })
        .collect()
}

/// Prints a document canonically.
#[must_use]
pub fn print(document: &Document) -> String {
    wesley_json_schema::to_document_string(&document.value)
}

fn object_of<'c>(catalog: &'c Catalog, name: &str) -> &'c Object {
    catalog
        .objects
        .get(name)
        .unwrap_or_else(|| unreachable!("the catalog holds every object a message reaches: {name}"))
}

fn document(catalog: &Catalog, m: &Message) -> Json {
    let o = object_of(catalog, &m.object);
    let mut doc = Map::new();
    doc.insert("$schema".into(), json!(wesley_json_schema::DIALECT));
    doc.insert(
        "$id".into(),
        json!(format!("{ID_BASE}{}/{}", m.id, m.version)),
    );
    doc.insert("title".into(), json!(m.title));
    if let Some(d) = &o.description {
        doc.insert("description".into(), json!(d));
    }
    if let Json::Object(body) = object_schema(catalog, o) {
        doc.extend(body);
    }
    Json::Object(doc)
}

fn object_schema(catalog: &Catalog, o: &Object) -> Json {
    let mut s = Map::new();
    s.insert("type".into(), json!("object"));
    let required: Vec<&str> = o
        .fields
        .iter()
        .filter(|f| f.required)
        .map(|f| f.name.as_str())
        .collect();
    if !required.is_empty() {
        s.insert("required".into(), json!(required));
    }
    if !o.open {
        s.insert("additionalProperties".into(), json!(false));
    }
    let mut properties = Map::new();
    for f in &o.fields {
        properties.insert(f.name.clone(), field_schema(catalog, f, true));
    }
    s.insert("properties".into(), Json::Object(properties));

    if let Some([a, b]) = &o.rules.exclusive {
        let mut props = Map::new();
        for n in [a, b] {
            let ty = field(o, n)
                .and_then(|f| json_type(&f.value))
                .unwrap_or("null");
            props.insert(n.clone(), json!({"type": ty}));
        }
        s.insert(
            "not".into(),
            json!({"type": "object", "required": [a, b], "properties": props}),
        );
    }
    if let Some(names) = &o.rules.any_non_empty {
        let branches: Vec<Json> = names
            .iter()
            .map(|n| json!({"type": "object", "required": [n], "properties": {n: {"type": "array", "minItems": 1}}}))
            .collect();
        s.insert("anyOf".into(), Json::Array(branches));
    }
    if let Some(w) = &o.rules.when {
        if let Json::Object(rule) = when_schema(catalog, o, w) {
            s.extend(rule);
        }
    }
    Json::Object(s)
}

fn field<'o>(o: &'o Object, name: &str) -> Option<&'o Field> {
    o.fields.iter().find(|f| f.name == name)
}

fn field_schema(catalog: &Catalog, f: &Field, describe: bool) -> Json {
    let mut s = value_schema(catalog, &f.value);
    if describe {
        if let (Some(d), Json::Object(map)) = (&f.description, &mut s) {
            map.insert("description".into(), json!(d));
        }
    }
    s
}

fn value_schema(catalog: &Catalog, v: &Value) -> Json {
    let mut s = Map::new();
    match v {
        Value::String(c) => {
            s.insert("type".into(), json!("string"));
            if let Some(p) = &c.pattern {
                s.insert("pattern".into(), json!(p));
            }
            if let Some(n) = c.min_length {
                s.insert("minLength".into(), json!(n));
            }
            if let Some(f) = &c.format {
                s.insert("format".into(), json!(f));
            }
        }
        Value::Integer { constraints: c, .. } => {
            s.insert("type".into(), json!("integer"));
            if let Some(n) = c.minimum {
                s.insert("minimum".into(), json!(n));
            }
            if let Some(n) = c.maximum {
                s.insert("maximum".into(), json!(n));
            }
            if let Some(n) = c.not_equal {
                s.insert("not".into(), json!({"const": n}));
            }
        }
        Value::Boolean => {
            s.insert("type".into(), json!("boolean"));
        }
        Value::Any => {}
        Value::JsonObject { min_properties } => {
            s.insert("type".into(), json!("object"));
            if let Some(n) = min_properties {
                s.insert("minProperties".into(), json!(n));
            }
        }
        Value::Const { value } => {
            s.insert("const".into(), value.clone());
        }
        Value::StringEnum { values, .. } => {
            s.insert("enum".into(), json!(values));
        }
        Value::MessageRef {
            object,
            id,
            version,
        } => {
            let mut r = object_schema(catalog, object_of(catalog, object));
            if let Some(Json::Object(props)) = r.get_mut("properties") {
                for (name, pinned) in [("id", id), ("version", version)] {
                    let description = props.get(name).and_then(|p| p.get("description")).cloned();
                    let mut p = Map::new();
                    p.insert("const".into(), json!(pinned));
                    if let Some(d) = description {
                        p.insert("description".into(), d);
                    }
                    props.insert(name.to_string(), Json::Object(p));
                }
            }
            return r;
        }
        Value::Object {
            name,
            min_properties,
        } => {
            let mut r = object_schema(catalog, object_of(catalog, name));
            if let (Some(n), Json::Object(map)) = (min_properties, &mut r) {
                map.insert("minProperties".into(), json!(n));
            }
            return r;
        }
        Value::List {
            item,
            min_items,
            max_items,
        } => {
            s.insert("type".into(), json!("array"));
            s.insert("items".into(), value_schema(catalog, item));
            if let Some(n) = min_items {
                s.insert("minItems".into(), json!(n));
            }
            if let Some(n) = max_items {
                s.insert("maxItems".into(), json!(n));
            }
        }
    }
    Json::Object(s)
}

/// `if`/`then`/`else` for a conditional rule (see [`When`]).
fn when_schema(catalog: &Catalog, o: &Object, w: &When) -> Json {
    // The trigger's parent: walk every step but the last.
    let mut parent = o;
    for step in &w.path[..w.path.len().saturating_sub(1)] {
        if let Some(Field {
            value: Value::Object { name, .. },
            ..
        }) = field(parent, step)
        {
            parent = object_of(catalog, name);
        }
    }
    let shape = |name: &str| {
        field(parent, name).map_or_else(|| json!({}), |f| field_schema(catalog, f, false))
    };

    let mut ifs = json!({"const": w.equals});
    for step in w.path.iter().rev() {
        ifs = if w.guarded {
            json!({"type": "object", "required": [step], "properties": {step: ifs}})
        } else {
            json!({"type": "object", "properties": {step: ifs}})
        };
    }

    let mut then = Map::new();
    then.insert("type".into(), json!("object"));
    if !w.require.is_empty() {
        then.insert("required".into(), json!(w.require));
    }
    let mut props = Map::new();
    for n in &w.require {
        props.insert(n.clone(), shape(n));
    }
    for (n, v) in &w.set {
        props.insert(n.clone(), json!({"const": v}));
    }
    then.insert("properties".into(), Json::Object(props));
    let mut then = Json::Object(then);
    // The forbidden form names each ancestor as required, or an object without the parent would
    // satisfy `then` vacuously and `not` would reject it.
    let mut forbidden = then.clone();
    for step in w.path[..w.path.len().saturating_sub(1)].iter().rev() {
        then = json!({"type": "object", "properties": {step: then}});
        forbidden = json!({"type": "object", "required": [step], "properties": {step: forbidden}});
    }

    let mut out = Map::new();
    out.insert("if".into(), ifs);
    out.insert("then".into(), then);
    if w.forbid_otherwise {
        out.insert("else".into(), json!({"not": forbidden}));
    }
    Json::Object(out)
}
