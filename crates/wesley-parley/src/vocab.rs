//! Parley's vocabulary: the `@parley_*` directives Parley declares in its own SDL, interpreted
//! into the [`model`](crate::model). This is the only place they get their meaning.
//!
//! Ported from flyingrobots/parley `scripts/sdl-to-json-schema.ts` at `c80771c`, where every rule
//! here was proven against Parley's 17 hand-written schemas.

use std::collections::{BTreeMap, HashSet};

use indexmap::IndexMap;
use serde_json::{Map, Value as Json};
use wesley_core::{Field as IrField, TypeDefinition, TypeKind, WesleyIR};

use crate::model::{
    Catalog, Field, IntegerConstraints, Message, Object, Rules, StringConstraints, Value, When,
};
use crate::Error;

/// The Wesley IR version this target reads.
pub const IR_VERSION: &str = "1.0.0";

/// Every directive in Parley's vocabulary, with the arguments it takes.
pub const DIRECTIVES: &[(&str, &[&str])] = &[
    ("parley_message", &["id", "version", "title"]),
    ("parley_int64", &[]),
    ("parley_scalar", &["json"]),
    ("parley_optional", &[]),
    ("parley_const", &["string", "int", "bool"]),
    ("parley_message_ref", &[]),
    ("parley_enum", &["values"]),
    ("parley_open", &[]),
    ("parley_closed", &[]),
    ("parley_pattern", &["regex"]),
    ("parley_minLength", &["value"]),
    ("parley_format", &["name"]),
    ("parley_minimum", &["value"]),
    ("parley_maximum", &["value"]),
    ("parley_not_equal", &["int"]),
    ("parley_minItems", &["value"]),
    ("parley_maxItems", &["value"]),
    ("parley_minProperties", &["value"]),
    ("parley_exclusive", &["fields"]),
    ("parley_any_non_empty", &["fields"]),
    (
        "parley_when",
        &[
            "path",
            "equals",
            "guarded",
            "require",
            "forbidOtherwise",
            "set",
        ],
    ),
];

const ON_OBJECT: &[&str] = &[
    "parley_message",
    "parley_open",
    "parley_closed",
    "parley_exclusive",
    "parley_any_non_empty",
    "parley_when",
];
const ON_SCALAR: &[&str] = &["parley_int64", "parley_scalar"];

fn fail<T>(message: impl Into<String>) -> Result<T, Error> {
    Err(Error::Vocabulary(message.into()))
}

type Args = Map<String, Json>;

/// One directive's arguments; a directive with no arguments lowers to `true`. Repeats are refused.
fn arg(directives: &IndexMap<String, Json>, name: &str, at: &str) -> Result<Option<Args>, Error> {
    let Some(value) = directives.get(name) else {
        return Ok(None);
    };
    let args = match value {
        Json::Bool(true) => Map::new(),
        Json::Object(map) => map.clone(),
        Json::Array(_) => return fail(format!("{at}: @{name} is repeated")),
        other => return fail(format!("{at}: @{name} has an unreadable value {other}")),
    };
    let known = DIRECTIVES
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(&[][..], |(_, a)| *a);
    for k in args.keys() {
        if !known.contains(&k.as_str()) {
            return fail(format!("{at}: @{name} has no argument {k}"));
        }
    }
    Ok(Some(args))
}

fn is_known(name: &str) -> bool {
    DIRECTIVES.iter().any(|(n, _)| *n == name)
}

fn string_arg(args: &Args, key: &str, at: &str) -> Result<String, Error> {
    match args.get(key) {
        Some(Json::String(s)) => Ok(s.clone()),
        _ => fail(format!("{at}: needs a string {key}")),
    }
}

fn u64_arg(args: &Args, key: &str, at: &str) -> Result<u64, Error> {
    args.get(key).and_then(Json::as_u64).map_or_else(
        || fail(format!("{at}: needs a non-negative integer {key}")),
        Ok,
    )
}

fn i64_arg(args: &Args, key: &str, at: &str) -> Result<i64, Error> {
    args.get(key)
        .and_then(Json::as_i64)
        .map_or_else(|| fail(format!("{at}: needs an integer {key}")), Ok)
}

fn names_arg(args: &Args, key: &str, at: &str) -> Result<Vec<String>, Error> {
    match args.get(key) {
        Some(Json::Array(items)) => items
            .iter()
            .map(|i| match i {
                Json::String(s) => Ok(s.clone()),
                _ => fail(format!("{at}: {key} holds names")),
            })
            .collect(),
        None => Ok(Vec::new()),
        _ => fail(format!("{at}: {key} is a list of names")),
    }
}

struct Context<'a> {
    types: BTreeMap<&'a str, &'a TypeDefinition>,
    objects: IndexMap<String, Object>,
    /// Object types being interpreted, outermost first, to refuse a cycle instead of recursing.
    visiting: Vec<String>,
}

struct Identity {
    id: String,
    version: String,
}

/// Interprets a Parley SDL's IR into a [`Catalog`].
///
/// # Errors
///
/// [`Error::Vocabulary`] for anything the vocabulary cannot render faithfully, naming the type,
/// field and directive.
pub fn catalog(ir: &WesleyIR) -> Result<Catalog, Error> {
    if ir.version != IR_VERSION {
        return fail(format!(
            "IR version {}, but this target reads {IR_VERSION}",
            ir.version
        ));
    }
    check_placement(ir)?;
    let mut cx = Context {
        types: ir.types.iter().map(|t| (t.name.as_str(), t)).collect(),
        objects: IndexMap::new(),
        visiting: Vec::new(),
    };
    let mut messages = Vec::new();
    let mut seen = HashSet::new();
    for t in &ir.types {
        let Some(msg) = arg(&t.directives, "parley_message", &t.name)? else {
            continue;
        };
        let identity = Identity {
            id: string_arg(&msg, "id", &t.name)?,
            version: string_arg(&msg, "version", &t.name)?,
        };
        check_message_id(&t.name, &identity.id)?;
        if !seen.insert(identity.id.clone()) {
            return fail(format!(
                "{}: message {} is declared twice",
                t.name, identity.id
            ));
        }
        let object = object(&mut cx, t, Some(&identity))?;
        cx.objects.insert(t.name.clone(), object);
        messages.push(Message {
            id: identity.id,
            version: identity.version,
            title: string_arg(&msg, "title", &t.name)?,
            object: t.name.clone(),
        });
    }
    cx.objects.sort_keys();
    Ok(Catalog {
        messages,
        objects: cx.objects,
    })
}

fn check_placement(ir: &WesleyIR) -> Result<(), Error> {
    for t in &ir.types {
        for name in t.directives.keys() {
            if !is_known(name) {
                return fail(format!("{}: unknown directive @{name}", t.name));
            }
            let allowed = match t.kind {
                TypeKind::Object => ON_OBJECT,
                TypeKind::Scalar => ON_SCALAR,
                _ => &[],
            };
            if !allowed.contains(&name.as_str()) {
                return fail(format!(
                    "{}: @{name} does not belong on a {:?}",
                    t.name, t.kind
                ));
            }
        }
        for f in &t.fields {
            for name in f.directives.keys() {
                if !is_known(name) {
                    return fail(format!("{}.{}: unknown directive @{name}", t.name, f.name));
                }
                if ON_OBJECT.contains(&name.as_str()) || ON_SCALAR.contains(&name.as_str()) {
                    return fail(format!(
                        "{}.{}: @{name} does not belong on a field",
                        t.name, f.name
                    ));
                }
            }
        }
    }
    Ok(())
}

fn object(
    cx: &mut Context<'_>,
    t: &TypeDefinition,
    message: Option<&Identity>,
) -> Result<Object, Error> {
    let open = arg(&t.directives, "parley_open", &t.name)?.is_some();
    let closed = arg(&t.directives, "parley_closed", &t.name)?.is_some();
    if open == closed {
        return fail(format!(
            "{}: an object type carries exactly one of @parley_open and @parley_closed",
            t.name
        ));
    }
    let mut fields: Vec<Field> = Vec::new();
    for f in &t.fields {
        if fields.iter().any(|x| x.name == f.name) {
            return fail(format!("{}.{} is declared twice", t.name, f.name));
        }
        fields.push(field(cx, t, f, message)?);
    }
    let mut out = Object {
        name: t.name.clone(),
        description: t.description.clone(),
        open,
        fields,
        rules: Rules::default(),
    };
    out.rules = rules(cx, t, &out, message)?;
    Ok(out)
}

fn find_field<'f>(
    fields: &'f [Field],
    owner: &str,
    name: &str,
    rule: &str,
) -> Result<&'f Field, Error> {
    fields.iter().find(|f| f.name == name).map_or_else(
        || {
            fail(format!(
                "@{rule} names {owner}.{name}, which does not exist"
            ))
        },
        Ok,
    )
}

/// The single JSON type a value has, if it has one.
pub(crate) fn json_type(value: &Value) -> Option<&'static str> {
    match value {
        Value::String(_) => Some("string"),
        Value::Integer { .. } => Some("integer"),
        Value::Boolean => Some("boolean"),
        Value::JsonObject { .. } | Value::Object { .. } | Value::MessageRef { .. } => {
            Some("object")
        }
        Value::List { .. } => Some("array"),
        Value::Any | Value::Const { .. } | Value::StringEnum { .. } => None,
    }
}

fn rules(
    cx: &mut Context<'_>,
    t: &TypeDefinition,
    o: &Object,
    message: Option<&Identity>,
) -> Result<Rules, Error> {
    let mut rules = Rules::default();
    if let Some(a) = arg(&t.directives, "parley_exclusive", &t.name)? {
        let names = names_arg(&a, "fields", &t.name)?;
        for n in &names {
            find_field(&o.fields, &t.name, n, "parley_exclusive")?;
        }
        // `not` of all of them present is "at most one" only for two names.
        let [a_name, b_name] = <[String; 2]>::try_from(names.clone()).map_or_else(
            |_| {
                fail(format!(
                    "@parley_exclusive on {} names {} fields; the rendering is exactly two",
                    t.name,
                    names.len()
                ))
            },
            Ok,
        )?;
        for n in [&a_name, &b_name] {
            if json_type(&find_field(&o.fields, &t.name, n, "parley_exclusive")?.value).is_none() {
                return fail(format!(
                    "@parley_exclusive names {}.{n}, which has no single JSON type",
                    t.name
                ));
            }
        }
        rules.exclusive = Some([a_name, b_name]);
    }
    if let Some(a) = arg(&t.directives, "parley_any_non_empty", &t.name)? {
        let names = names_arg(&a, "fields", &t.name)?;
        for n in &names {
            let f = find_field(&o.fields, &t.name, n, "parley_any_non_empty")?;
            if !matches!(f.value, Value::List { .. }) {
                return fail(format!(
                    "@parley_any_non_empty names {}.{n}, which is not a list",
                    t.name
                ));
            }
        }
        rules.any_non_empty = Some(names);
    }
    if let Some(a) = arg(&t.directives, "parley_when", &t.name)? {
        rules.when = Some(when(cx, t, o, &a, message)?);
    }
    Ok(rules)
}

fn when(
    cx: &mut Context<'_>,
    t: &TypeDefinition,
    o: &Object,
    a: &Args,
    message: Option<&Identity>,
) -> Result<When, Error> {
    let path = names_arg(a, "path", &t.name)?;
    if path.is_empty() {
        return fail(format!("{}: @parley_when needs a non-empty path", t.name));
    }
    let Some(Json::Bool(guarded)) = a.get("guarded") else {
        return fail(format!(
            "{}: @parley_when needs guarded: true or false",
            t.name
        ));
    };
    // Walk the path; the last step is the trigger, and its parent holds the fields the rule names.
    let mut parent_fields: Vec<Field> = o.fields.clone();
    let mut parent_name = t.name.clone();
    for (i, step) in path.iter().enumerate() {
        let f = find_field(&parent_fields, &parent_name, step, "parley_when")?.clone();
        if i + 1 < path.len() {
            let Value::Object { name, .. } = &f.value else {
                return fail(format!(
                    "@parley_when path {}: {parent_name}.{step} is not an object",
                    path.join(".")
                ));
            };
            let def = *cx
                .types
                .get(name.as_str())
                .ok_or_else(|| Error::Vocabulary(format!("unknown type {name}")))?;
            ensure_object(cx, def)?;
            parent_fields = cx.objects[name.as_str()].fields.clone();
            parent_name = name.clone();
        }
    }
    let _ = message;
    let shape = |name: &str| -> Result<Value, Error> {
        Ok(
            find_field(&parent_fields, &parent_name, name, "parley_when")?
                .value
                .clone(),
        )
    };
    let trigger = shape(path.last().map_or("", String::as_str))?;
    let equals = a.get("equals").cloned().unwrap_or(Json::Null);
    check_value(
        &equals,
        &trigger,
        &format!("@parley_when on {}: equals", t.name),
    )?;

    let require = names_arg(a, "require", &t.name)?;
    let forbid_otherwise = matches!(a.get("forbidOtherwise"), Some(Json::Bool(true)));
    if forbid_otherwise && require.len() != 1 {
        return fail(format!("{}: @parley_when forbidOtherwise forbids one required field; `not` of several would only forbid all of them together", t.name));
    }
    let set: IndexMap<String, Json> = match a.get("set") {
        None | Some(Json::Null) => IndexMap::new(),
        Some(Json::Object(m)) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Some(_) => return fail(format!("{}: @parley_when set is an object", t.name)),
    };
    if require.is_empty() && set.is_empty() {
        return fail(format!("{}: @parley_when requires or sets nothing", t.name));
    }
    for n in &require {
        shape(n)?;
    }
    for (n, v) in &set {
        check_value(
            v,
            &shape(n)?,
            &format!("@parley_when on {}: set {n}", t.name),
        )?;
    }
    if forbid_otherwise && !set.is_empty() {
        return fail(format!(
            "{}: @parley_when forbidOtherwise applies to required fields only",
            t.name
        ));
    }
    Ok(When {
        path,
        equals,
        guarded: *guarded,
        require,
        forbid_otherwise,
        set,
    })
}

/// A literal a rule compares with must be a value the field can hold.
fn check_value(v: &Json, field: &Value, at: &str) -> Result<(), Error> {
    let ok = match field {
        Value::Const { value } => v == value,
        Value::StringEnum { values, .. } => {
            v.as_str().is_some_and(|s| values.iter().any(|x| x == s))
        }
        Value::String(c) => match v.as_str() {
            Some(s) => {
                if c.pattern.is_some() {
                    return fail(format!(
                        "{at} is {v}, but a literal cannot be checked against a pattern here"
                    ));
                }
                c.min_length.is_none_or(|m| s.chars().count() as u64 >= m)
            }
            None => false,
        },
        Value::Boolean => v.is_boolean(),
        Value::Integer { constraints: c, .. } => match v.as_i64() {
            Some(n) => {
                c.minimum.is_none_or(|m| n >= m)
                    && c.maximum.is_none_or(|m| n <= m)
                    && c.not_equal != Some(n)
            }
            None => false,
        },
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        fail(format!("{at} is {v}, which the field cannot hold"))
    }
}

fn field(
    cx: &mut Context<'_>,
    t: &TypeDefinition,
    f: &IrField,
    message: Option<&Identity>,
) -> Result<Field, Error> {
    let at = format!("{}.{}", t.name, f.name);
    let d = &f.directives;
    let r = &f.r#type;
    let optional = arg(d, "parley_optional", &at)?.is_some();
    if r.nullable && !optional {
        return fail(format!("{at} is nullable but not marked @parley_optional; null is never a legal Parley value, so an optional field is absent when unset"));
    }
    if !r.nullable && optional {
        return fail(format!("{at} is non-null and marked @parley_optional"));
    }
    if r.list_wrappers.len() > 1 {
        return fail(format!(
            "{at}: a nested list type has no JSON Schema rendering here"
        ));
    }
    if r.is_list && r.list_item_nullable != Some(false) {
        return fail(format!("{at}: list items are never null"));
    }

    let defining: Vec<&str> = ["parley_const", "parley_message_ref", "parley_enum"]
        .into_iter()
        .filter(|n| d.contains_key(*n))
        .collect();
    if defining.len() > 1 {
        return fail(format!(
            "{at}: @{} and @{} both define the value; a field takes one",
            defining[0], defining[1]
        ));
    }
    let mut value = if let Some(lit) = arg(d, "parley_const", &at)? {
        if lit.len() != 1 {
            return fail(format!(
                "{at}: @parley_const takes exactly one of string, int, bool"
            ));
        }
        let (kind, literal) = lit
            .iter()
            .next()
            .map_or_else(|| unreachable!(), |(k, v)| (k.clone(), v.clone()));
        let fits = match kind.as_str() {
            "string" => r.base == "String" && literal.is_string(),
            "int" => r.base == "Int" && literal.is_i64(),
            "bool" => r.base == "Boolean" && literal.is_boolean(),
            _ => false,
        };
        if !fits || r.is_list {
            return fail(format!(
                "{at}: @parley_const({kind}:) does not fit {}",
                r.base
            ));
        }
        Value::Const { value: literal }
    } else if arg(d, "parley_message_ref", &at)?.is_some() {
        let Some(identity) = message else {
            return fail(format!("{at}: @parley_message_ref outside a message"));
        };
        if r.is_list {
            return fail(format!(
                "{at}: @parley_message_ref needs a single object, not a list"
            ));
        }
        message_ref(cx, &at, &r.base, identity)?
    } else if let Some(e) = arg(d, "parley_enum", &at)? {
        if r.base != "String" || r.is_list {
            return fail(format!(
                "{at}: @parley_enum is for a String field whose values are not GraphQL names"
            ));
        }
        Value::StringEnum {
            values: names_arg(&e, "values", &at)?,
            type_name: None,
        }
    } else {
        let item = base(cx, &at, &r.base)?;
        if r.is_list {
            Value::List {
                item: Box::new(item),
                min_items: None,
                max_items: None,
            }
        } else {
            item
        }
    };

    // A string or integer constraint on a list field constrains its items; a list constraint, the list.
    let is_list = matches!(value, Value::List { .. });
    {
        let target = match &mut value {
            Value::List { item, .. } => item.as_mut(),
            other => other,
        };
        for (name, key) in [
            ("parley_pattern", "regex"),
            ("parley_minLength", "value"),
            ("parley_format", "name"),
        ] {
            let Some(c) = arg(d, name, &at)? else {
                continue;
            };
            let Value::String(sc) = target else {
                return fail(format!("{at}: @{name} needs a string"));
            };
            set_string_constraint(sc, name, &c, key, &at)?;
        }
        for name in ["parley_minimum", "parley_maximum"] {
            let Some(c) = arg(d, name, &at)? else {
                continue;
            };
            let Value::Integer { constraints, .. } = target else {
                return fail(format!("{at}: @{name} needs an integer"));
            };
            let v = i64_arg(&c, "value", &at)?;
            if name == "parley_minimum" {
                constraints.minimum = Some(v);
            } else {
                constraints.maximum = Some(v);
            }
        }
        if let Some(c) = arg(d, "parley_not_equal", &at)? {
            let Value::Integer { constraints, .. } = target else {
                return fail(format!(
                    "{at}: @parley_not_equal(int:) needs an integer field"
                ));
            };
            constraints.not_equal = Some(i64_arg(&c, "int", &at)?);
        }
    }
    for name in ["parley_minItems", "parley_maxItems"] {
        let Some(c) = arg(d, name, &at)? else {
            continue;
        };
        let Value::List {
            min_items,
            max_items,
            ..
        } = &mut value
        else {
            return fail(format!("{at}: @{name} needs a list"));
        };
        let v = u64_arg(&c, "value", &at)?;
        if name == "parley_minItems" {
            *min_items = Some(v);
        } else {
            *max_items = Some(v);
        }
    }
    if let Some(c) = arg(d, "parley_minProperties", &at)? {
        let v = u64_arg(&c, "value", &at)?;
        match &mut value {
            Value::Object { min_properties, .. } | Value::JsonObject { min_properties }
                if !is_list =>
            {
                *min_properties = Some(v);
            }
            _ => return fail(format!("{at}: @parley_minProperties needs an object")),
        }
    }
    Ok(Field {
        name: f.name.clone(),
        description: f.description.clone(),
        required: !r.nullable,
        value,
    })
}

fn set_string_constraint(
    sc: &mut StringConstraints,
    name: &str,
    c: &Args,
    key: &str,
    at: &str,
) -> Result<(), Error> {
    match name {
        "parley_pattern" => sc.pattern = Some(string_arg(c, key, at)?),
        "parley_minLength" => sc.min_length = Some(u64_arg(c, key, at)?),
        _ => sc.format = Some(string_arg(c, key, at)?),
    }
    Ok(())
}

/// `_schema`: a closed `{id, version}` pinned by literal to the enclosing message's identity.
fn message_ref(
    cx: &mut Context<'_>,
    at: &str,
    base: &str,
    identity: &Identity,
) -> Result<Value, Error> {
    let Some(def) = cx
        .types
        .get(base)
        .copied()
        .filter(|t| t.kind == TypeKind::Object)
    else {
        return fail(format!("{at}: @parley_message_ref needs an object type"));
    };
    let names: Vec<&str> = def.fields.iter().map(|f| f.name.as_str()).collect();
    if names != ["id", "version"] || arg(&def.directives, "parley_closed", &def.name)?.is_none() {
        return fail(format!(
            "{at}: @parley_message_ref needs a closed type of exactly id and version, not {}({})",
            def.name,
            names.join(",")
        ));
    }
    for f in &def.fields {
        if f.r#type.base != "String"
            || f.r#type.is_list
            || f.r#type.nullable
            || !f.directives.is_empty()
        {
            return fail(format!("{}.{} must be a bare String!", def.name, f.name));
        }
    }
    ensure_object(cx, def)?;
    Ok(Value::MessageRef {
        object: def.name.clone(),
        id: identity.id.clone(),
        version: identity.version.clone(),
    })
}

fn ensure_object(cx: &mut Context<'_>, def: &TypeDefinition) -> Result<(), Error> {
    if cx.objects.contains_key(&def.name) {
        return Ok(());
    }
    if let Some(start) = cx.visiting.iter().position(|n| *n == def.name) {
        let mut cycle = cx.visiting[start..].to_vec();
        cycle.push(def.name.clone());
        return fail(format!(
            "{}: the object types form a cycle ({}), which has no inline JSON Schema rendering",
            def.name,
            cycle.join(" -> ")
        ));
    }
    cx.visiting.push(def.name.clone());
    let o = object(cx, def, None);
    cx.visiting.pop();
    cx.objects.insert(def.name.clone(), o?);
    Ok(())
}

fn base(cx: &mut Context<'_>, at: &str, name: &str) -> Result<Value, Error> {
    match name {
        "String" => return Ok(Value::String(StringConstraints::default())),
        "Boolean" => return Ok(Value::Boolean),
        "Int" => {
            return Ok(Value::Integer {
                bits: 32,
                constraints: IntegerConstraints::default(),
            })
        }
        _ => {}
    }
    let Some(t) = cx.types.get(name).copied() else {
        return fail(format!("{at}: unknown type {name}"));
    };
    match t.kind {
        TypeKind::Enum => Ok(Value::StringEnum {
            values: t.enum_values.clone(),
            type_name: Some(t.name.clone()),
        }),
        TypeKind::Object => {
            if t.directives.contains_key("parley_message") {
                return fail(format!("{at}: a message type {name} cannot be nested"));
            }
            if t.description.is_some() {
                return fail(format!("{name}: a nested type's description renders nowhere; describe the field that uses it"));
            }
            ensure_object(cx, t)?;
            Ok(Value::Object {
                name: name.to_string(),
                min_properties: None,
            })
        }
        TypeKind::Scalar => {
            if arg(&t.directives, "parley_int64", name)?.is_some() {
                return Ok(Value::Integer {
                    bits: 64,
                    constraints: IntegerConstraints::default(),
                });
            }
            match arg(&t.directives, "parley_scalar", name)?
                .as_ref()
                .and_then(|a| a.get("json"))
                .and_then(Json::as_str)
            {
                Some("any") => Ok(Value::Any),
                Some("object") => Ok(Value::JsonObject { min_properties: None }),
                _ => fail(format!("{at}: scalar {name} declares no JSON kind (@parley_int64, or @parley_scalar(json: \"any\" | \"object\"))")),
            }
        }
        other => fail(format!(
            "{at}: {other:?} {name} has no JSON Schema rendering here"
        )),
    }
}

/// A message id names a file (`<id>.schema.json`), so it must be one safe path component.
fn check_message_id(type_name: &str, id: &str) -> Result<(), Error> {
    let safe = !id.is_empty()
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if safe {
        Ok(())
    } else {
        fail(format!(
            "{type_name}: message id {id:?} is not one path component of letters, digits, '.', '_' and '-' that does not start with '.'"
        ))
    }
}
