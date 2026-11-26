//! Parley's vocabulary refuses what it cannot render faithfully. Ported from flyingrobots/parley
//! `scripts/sdl-agreement.test.ts` (SDL-6) at `c80771c`, where the same rules were proven.

use serde_json::{json, Value};
use wesley_core::WesleyIR;
use wesley_parley::{catalog, json_schema};

/// A minimal well-formed IR: one open message with a closed `_schema`.
fn tiny() -> Value {
    json!({
        "version": "1.0.0",
        "types": [
            { "name": "SchemaRef", "kind": "OBJECT", "directives": { "parley_closed": true }, "fields": [
                { "name": "id", "type": { "base": "String", "nullable": false, "isList": false }, "directives": {} },
                { "name": "version", "type": { "base": "String", "nullable": false, "isList": false }, "directives": {} }
            ] },
            { "name": "Tiny", "kind": "OBJECT", "description": "A tiny message.",
              "directives": { "parley_message": { "id": "parley.tiny", "version": "1.0.0", "title": "parley tiny" }, "parley_open": true },
              "fields": [
                { "name": "event", "type": { "base": "String", "nullable": false, "isList": false }, "directives": { "parley_const": { "string": "tiny" } } },
                { "name": "note", "type": { "base": "String", "nullable": true, "isList": false }, "directives": { "parley_optional": true } },
                { "name": "_schema", "type": { "base": "SchemaRef", "nullable": false, "isList": false }, "directives": { "parley_message_ref": true } }
            ] }
        ]
    })
}

fn ir(v: &Value) -> WesleyIR {
    serde_json::from_value(v.clone()).expect("a well-formed IR")
}

fn tiny_type(v: &mut Value) -> &mut Value {
    &mut v["types"][1]
}

fn refusal(v: &Value) -> String {
    match catalog(&ir(v)) {
        Ok(_) => panic!("expected a refusal"),
        Err(e) => e.to_string(),
    }
}

fn schema(v: &Value) -> Value {
    let docs = json_schema::documents(&catalog(&ir(v)).expect("valid"));
    docs.into_iter()
        .find(|d| d.id == "parley.tiny")
        .expect("tiny")
        .value
}

#[test]
fn the_tiny_ir_is_valid_so_each_refusal_below_is_its_one_change() {
    let s = schema(&tiny());
    assert_eq!(s["properties"]["event"], json!({"const": "tiny"}));
    assert_eq!(s["required"], json!(["event", "_schema"]));
    assert_eq!(
        s["properties"]["_schema"]["properties"]["id"],
        json!({"const": "parley.tiny"})
    );
}

#[test]
fn an_unknown_directive() {
    let mut v = tiny();
    tiny_type(&mut v)["fields"][1]["directives"]["parley_maxLenght"] = json!({"value": 3});
    assert!(refusal(&v).contains("parley_maxLenght"), "{}", refusal(&v));
}

#[test]
fn a_nullable_field_not_marked_optional() {
    let mut v = tiny();
    tiny_type(&mut v)["fields"][1]["directives"] = json!({});
    let e = refusal(&v);
    assert!(
        e.contains("Tiny.note") && e.contains("@parley_optional"),
        "{e}"
    );
}

#[test]
fn an_object_type_neither_open_nor_closed() {
    let mut v = tiny();
    tiny_type(&mut v)["directives"]
        .as_object_mut()
        .expect("map")
        .remove("parley_open");
    let e = refusal(&v);
    assert!(
        e.contains("Tiny") && e.contains("@parley_open") && e.contains("@parley_closed"),
        "{e}"
    );
}

#[test]
fn a_rule_naming_a_field_the_type_does_not_have() {
    let mut v = tiny();
    tiny_type(&mut v)["directives"]["parley_exclusive"] = json!({"fields": ["note", "nope"]});
    let e = refusal(&v);
    assert!(
        e.contains("@parley_exclusive") && e.contains("Tiny.nope"),
        "{e}"
    );
}

#[test]
fn an_unknown_argument_on_a_known_directive() {
    let mut v = tiny();
    tiny_type(&mut v)["fields"][1]["directives"]["parley_minLength"] =
        json!({"value": 2, "maximum": 3});
    let e = refusal(&v);
    assert!(
        e.contains("Tiny.note") && e.contains("@parley_minLength") && e.contains("maximum"),
        "{e}"
    );
    tiny_type(&mut v)["fields"][1]["directives"]["parley_minLength"] = json!({"value": 2});
    assert_eq!(schema(&v)["properties"]["note"]["minLength"], 2);
}

#[test]
fn a_field_declared_twice() {
    let mut v = tiny();
    let mut dup = tiny_type(&mut v)["fields"][1].clone();
    dup["directives"] = json!({"parley_optional": true, "parley_minLength": {"value": 2}});
    tiny_type(&mut v)["fields"]
        .as_array_mut()
        .expect("fields")
        .insert(0, dup);
    let e = refusal(&v);
    assert!(e.contains("Tiny.note") && e.contains("twice"), "{e}");
}

#[test]
fn a_nested_list_type() {
    let mut v = tiny();
    tiny_type(&mut v)["fields"][1] = json!({ "name": "note", "directives": {},
        "type": { "base": "String", "nullable": false, "isList": true, "listItemNullable": false,
                  "listWrappers": [{ "nullable": false }, { "nullable": false }] } });
    let e = refusal(&v);
    assert!(e.contains("Tiny.note") && e.contains("nested list"), "{e}");
}

#[test]
fn a_list_valued_message_ref() {
    let mut v = tiny();
    tiny_type(&mut v)["fields"][2]["type"] = json!({ "base": "SchemaRef", "nullable": false, "isList": true, "listItemNullable": false });
    let e = refusal(&v);
    assert!(
        e.contains("Tiny._schema") && e.contains("@parley_message_ref") && e.contains("list"),
        "{e}"
    );
}

#[test]
fn a_list_typed_member_of_the_message_reference_type() {
    let mut v = tiny();
    v["types"][0]["fields"][0]["type"] =
        json!({ "base": "String", "nullable": false, "isList": true, "listItemNullable": false });
    let e = refusal(&v);
    assert!(
        e.contains("SchemaRef.id") && e.contains("bare String!"),
        "{e}"
    );
}

#[test]
fn exclusive_over_three_names_which_not_of_all_present_would_under_enforce() {
    let mut v = tiny();
    for n in ["a", "b", "c"] {
        tiny_type(&mut v)["fields"].as_array_mut().expect("fields").push(json!({
            "name": n, "type": { "base": "String", "nullable": true, "isList": false }, "directives": { "parley_optional": true }
        }));
    }
    tiny_type(&mut v)["directives"]["parley_exclusive"] = json!({"fields": ["a", "b"]});
    assert!(catalog(&ir(&v)).is_ok());
    tiny_type(&mut v)["directives"]["parley_exclusive"] = json!({"fields": ["a", "b", "c"]});
    let e = refusal(&v);
    assert!(
        e.contains("@parley_exclusive") && e.contains("Tiny") && e.contains("exactly two"),
        "{e}"
    );
}

/// A message whose optional `detail` object carries a rule on `detail.enabled`.
fn when_ir(rule: &Value) -> Value {
    let mut v = tiny();
    v["types"].as_array_mut().expect("types").push(json!({ "name": "Detail", "kind": "OBJECT", "directives": { "parley_open": true }, "fields": [
        { "name": "enabled", "type": { "base": "Boolean", "nullable": false, "isList": false }, "directives": {} },
        { "name": "reason", "type": { "base": "String", "nullable": true, "isList": false }, "directives": { "parley_optional": true } },
        { "name": "note2", "type": { "base": "String", "nullable": true, "isList": false }, "directives": { "parley_optional": true } }
    ] }));
    tiny_type(&mut v)["fields"].as_array_mut().expect("fields").push(json!({
        "name": "detail", "type": { "base": "Detail", "nullable": true, "isList": false }, "directives": { "parley_optional": true }
    }));
    let mut when = json!({ "path": ["detail", "enabled"], "equals": true, "guarded": true, "forbidOtherwise": true });
    for (k, val) in rule.as_object().expect("rule") {
        when[k] = val.clone();
    }
    tiny_type(&mut v)["directives"]["parley_when"] = when;
    v
}

#[test]
fn a_guarded_forbid_rule_renders_if_then_else_with_the_parent_required() {
    let s = schema(&when_ir(&json!({"require": ["reason"]})));
    assert_eq!(
        s["if"],
        json!({"type": "object", "required": ["detail"], "properties": {"detail": {"type": "object", "required": ["enabled"], "properties": {"enabled": {"const": true}}}}})
    );
    assert_eq!(
        s["then"],
        json!({"type": "object", "properties": {"detail": {"type": "object", "required": ["reason"], "properties": {"reason": {"type": "string"}}}}})
    );
    assert_eq!(
        s["else"],
        json!({"not": {"type": "object", "required": ["detail"], "properties": {"detail": {"type": "object", "required": ["reason"], "properties": {"reason": {"type": "string"}}}}}})
    );
}

#[test]
fn forbid_otherwise_with_two_required_fields_which_not_of_both_would_under_enforce() {
    let e = refusal(&when_ir(&json!({"require": ["reason", "note2"]})));
    assert!(e.contains("forbidOtherwise") && e.contains("one"), "{e}");
}

#[test]
fn a_rule_literal_that_violates_the_fields_own_constraints() {
    let mut v = when_ir(
        &json!({"guarded": true, "forbidOtherwise": false, "set": {"n": -1}, "require": []}),
    );
    v["types"][2]["fields"]
        .as_array_mut()
        .expect("fields")
        .push(json!({
            "name": "n", "type": { "base": "Int", "nullable": true, "isList": false },
            "directives": { "parley_optional": true, "parley_minimum": { "value": 0 } }
        }));
    let e = refusal(&v);
    assert!(
        e.contains("set n") && e.contains("-1") && e.contains("cannot hold"),
        "{e}"
    );
    tiny_type(&mut v)["directives"]["parley_when"]["set"] = json!({"n": 0});
    assert!(catalog(&ir(&v)).is_ok());
}

#[test]
fn an_ir_of_another_version_is_refused() {
    let mut v = tiny();
    v["version"] = json!("2.0.0");
    assert!(refusal(&v).contains("2.0.0"));
}

#[test]
fn a_cycle_of_nested_objects_is_refused_not_recursed() {
    let mut v = tiny();
    v["types"].as_array_mut().expect("types").push(json!({ "name": "Node", "kind": "OBJECT", "directives": { "parley_open": true }, "fields": [
        { "name": "next", "type": { "base": "Node", "nullable": true, "isList": false }, "directives": { "parley_optional": true } }
    ] }));
    tiny_type(&mut v)["fields"].as_array_mut().expect("fields").push(json!({
        "name": "node", "type": { "base": "Node", "nullable": true, "isList": false }, "directives": { "parley_optional": true }
    }));
    let e = refusal(&v);
    assert!(e.contains("Node") && e.contains("cycle"), "{e}");
}

#[test]
fn a_message_id_that_is_not_a_single_path_component_is_refused() {
    for id in ["../victim", "/abs", "a/b", "a\\b", "..", ".", "", ".hidden"] {
        let mut v = tiny();
        tiny_type(&mut v)["directives"]["parley_message"]["id"] = json!(id);
        let e = refusal(&v);
        assert!(e.contains("message id"), "{id:?}: {e}");
    }
}

#[test]
fn two_value_directives_on_one_field_are_refused() {
    let mut v = tiny();
    tiny_type(&mut v)["fields"][0]["directives"]["parley_enum"] = json!({"values": ["b"]});
    let e = refusal(&v);
    assert!(
        e.contains("Tiny.event") && e.contains("@parley_const") && e.contains("@parley_enum"),
        "{e}"
    );
}

#[test]
fn a_rule_literal_is_measured_in_characters_as_json_schema_does() {
    let mut v = when_ir(
        &json!({"guarded": true, "forbidOtherwise": false, "set": {"label": "\u{1F600}"}, "require": []}),
    );
    v["types"][2]["fields"]
        .as_array_mut()
        .expect("fields")
        .push(json!({
            "name": "label", "type": { "base": "String", "nullable": true, "isList": false },
            "directives": { "parley_optional": true, "parley_minLength": { "value": 2 } }
        }));
    let e = refusal(&v);
    assert!(e.contains("set label") && e.contains("cannot hold"), "{e}");
}

#[test]
fn an_empty_any_non_empty_list_is_refused() {
    let mut v = tiny();
    tiny_type(&mut v)["directives"]["parley_any_non_empty"] = json!({"fields": []});
    let e = refusal(&v);
    assert!(
        e.contains("@parley_any_non_empty") && e.contains("at least one"),
        "{e}"
    );
}

#[test]
fn an_exclusive_rule_naming_one_field_twice_is_refused() {
    let mut v = tiny();
    tiny_type(&mut v)["directives"]["parley_exclusive"] = json!({"fields": ["note", "note"]});
    let e = refusal(&v);
    assert!(
        e.contains("@parley_exclusive") && e.contains("twice"),
        "{e}"
    );
}

#[test]
fn a_when_rule_requiring_one_field_twice_is_refused() {
    let e = refusal(&when_ir(
        &json!({"require": ["reason", "reason"], "forbidOtherwise": false}),
    ));
    assert!(
        e.contains("@parley_when") && e.contains("reason") && e.contains("twice"),
        "{e}"
    );
}
