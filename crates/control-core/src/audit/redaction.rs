use serde_json::{Value, json};
pub fn summarize(value: &Value) -> Value {
    let o = value.as_object();
    let mut types: Vec<&str> = o
        .into_iter()
        .flat_map(|o| o.values())
        .map(|v| match v {
            Value::Null => "NoneType",
            Value::Bool(_) => "bool",
            Value::Number(n) if n.is_i64() || n.is_u64() => "int",
            Value::Number(_) => "float",
            Value::String(_) => "str",
            Value::Array(_) => "list",
            Value::Object(_) => "dict",
        })
        .collect();
    types.sort();
    types.dedup();
    json!({"parameter_count":o.map_or(0,|o|o.len()),"value_types":types,"values_recorded":false})
}
