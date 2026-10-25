//! Response body extraction and schema example generation.

use serde_json::{Map, Value, json};

/// Where a generated response body came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodySource {
    Example,
    Schema,
    Empty,
}

impl BodySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            BodySource::Example => "example",
            BodySource::Schema => "schema",
            BodySource::Empty => "empty",
        }
    }
}

/// Resolve a response's body in priority order: explicit example ->
/// schema-generated example -> empty.
pub(super) fn response_body(doc: &Value, resp: &Value) -> (Option<String>, Vec<u8>, BodySource) {
    let Some(content) = resp.get("content").and_then(Value::as_object) else {
        return (None, Vec::new(), BodySource::Empty);
    };
    // Prefer application/json; otherwise the first declared media type.
    let (ct, media) = content
        .get_key_value("application/json")
        .or_else(|| content.iter().next())
        .map(|(k, v)| (k.clone(), v))
        .unwrap_or_else(|| ("application/json".to_string(), &Value::Null));

    // 1. explicit `example`.
    if let Some(example) = media.get("example") {
        return (Some(ct), to_bytes(example), BodySource::Example);
    }
    // 2. first of `examples`.
    if let Some(examples) = media.get("examples").and_then(Value::as_object)
        && let Some((_, first)) = examples.iter().next()
        && let Some(value) = first.get("value")
    {
        return (Some(ct), to_bytes(value), BodySource::Example);
    }
    // 3. schema-generated example.
    if let Some(schema) = media.get("schema") {
        let mut visited = Vec::new();
        let generated = gen_example(doc, schema, &mut visited, 0);
        return (Some(ct), to_bytes(&generated), BodySource::Schema);
    }
    (Some(ct), Vec::new(), BodySource::Empty)
}

fn to_bytes(v: &Value) -> Vec<u8> {
    match v {
        // A bare string body is emitted as-is (not JSON-quoted).
        Value::String(s) => s.clone().into_bytes(),
        Value::Null => Vec::new(),
        other => serde_json::to_vec(other).unwrap_or_default(),
    }
}

/// Generate an example `Value` from a JSON Schema fragment, resolving local
/// `$ref`s against `doc` and guarding against cycles / deep nesting.
pub fn gen_example(doc: &Value, schema: &Value, visited: &mut Vec<String>, depth: u8) -> Value {
    if depth > 24 {
        return Value::Null;
    }
    let schema = match schema.as_object() {
        Some(o) => o,
        None => return Value::Null,
    };

    // $ref resolution.
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        if visited.iter().any(|r| r == reference) {
            return Value::Null; // cycle
        }
        visited.push(reference.to_string());
        let resolved = resolve_ref(doc, reference);
        let out = match resolved {
            Some(r) => gen_example(doc, r, visited, depth + 1),
            None => Value::Null,
        };
        visited.pop();
        return out;
    }

    // Explicit example / default / enum win over structural generation.
    if let Some(example) = schema.get("example") {
        return example.clone();
    }
    if let Some(default) = schema.get("default") {
        return default.clone();
    }
    if let Some(Value::Array(vals)) = schema.get("enum")
        && let Some(first) = vals.first()
    {
        return first.clone();
    }

    // Composition keywords.
    if let Some(Value::Array(all)) = schema.get("allOf") {
        let mut merged = Map::new();
        for sub in all {
            if let Value::Object(obj) = gen_example(doc, sub, visited, depth + 1) {
                merged.extend(obj);
            }
        }
        return Value::Object(merged);
    }
    for key in ["oneOf", "anyOf"] {
        if let Some(Value::Array(variants)) = schema.get(key)
            && let Some(first) = variants.first()
        {
            return gen_example(doc, first, visited, depth + 1);
        }
    }

    match type_of(schema) {
        Some("object") => gen_object(doc, schema, visited, depth),
        Some("array") => {
            let item = schema
                .get("items")
                .map(|items| gen_example(doc, items, visited, depth + 1))
                .unwrap_or(Value::Null);
            Value::Array(vec![item])
        }
        Some("string") => gen_string(schema),
        Some("integer") => json!(0),
        Some("number") => json!(0.0),
        Some("boolean") => json!(true),
        Some("null") => Value::Null,
        _ => {
            // No explicit type but properties present => treat as object.
            if schema.contains_key("properties") {
                gen_object(doc, schema, visited, depth)
            } else {
                Value::Null
            }
        }
    }
}

/// OpenAPI 3.1 allows `type` to be an array; take the first non-null type.
fn type_of(schema: &Map<String, Value>) -> Option<&str> {
    match schema.get("type") {
        Some(Value::String(s)) => Some(s.as_str()),
        Some(Value::Array(arr)) => arr.iter().filter_map(Value::as_str).find(|t| *t != "null"),
        _ => None,
    }
}

fn gen_object(
    doc: &Value,
    schema: &Map<String, Value>,
    visited: &mut Vec<String>,
    depth: u8,
) -> Value {
    let mut obj = Map::new();
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        for (name, prop_schema) in props {
            obj.insert(
                name.clone(),
                gen_example(doc, prop_schema, visited, depth + 1),
            );
        }
    }
    Value::Object(obj)
}

fn gen_string(schema: &Map<String, Value>) -> Value {
    match schema.get("format").and_then(Value::as_str) {
        Some("date-time") => json!("2024-01-01T00:00:00Z"),
        Some("date") => json!("2024-01-01"),
        Some("email") => json!("user@example.com"),
        Some("uuid") => json!("00000000-0000-0000-0000-000000000000"),
        Some("uri") => json!("https://example.com"),
        _ => json!("string"),
    }
}

/// Resolve a local JSON pointer `$ref` (`#/components/schemas/Foo`).
pub(super) fn resolve_ref<'a>(doc: &'a Value, reference: &str) -> Option<&'a Value> {
    let pointer = reference.strip_prefix('#')?;
    doc.pointer(pointer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_example_object() {
        let doc = json!({});
        let schema = json!({
            "type": "object",
            "properties": {
                "id": {"type": "integer"},
                "name": {"type": "string"},
                "active": {"type": "boolean"}
            }
        });
        let mut v = Vec::new();
        let out = gen_example(&doc, &schema, &mut v, 0);
        assert_eq!(out, json!({"id": 0, "name": "string", "active": true}));
    }

    #[test]
    fn resolves_ref_and_breaks_cycle() {
        let doc = json!({
            "components": {"schemas": {
                "Node": {"type": "object", "properties": {
                    "next": {"$ref": "#/components/schemas/Node"},
                    "label": {"type": "string"}
                }}
            }}
        });
        let schema = json!({"$ref": "#/components/schemas/Node"});
        let mut v = Vec::new();
        let out = gen_example(&doc, &schema, &mut v, 0);
        // Cycle on `next` yields null; `label` still generated.
        assert_eq!(out, json!({"next": null, "label": "string"}));
    }

    #[test]
    fn example_wins_over_schema() {
        let doc = json!({});
        let schema = json!({"type": "string", "example": "hello"});
        let mut v = Vec::new();
        assert_eq!(gen_example(&doc, &schema, &mut v, 0), json!("hello"));
    }
}
