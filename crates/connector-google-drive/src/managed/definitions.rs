use control_core::connector::Tool;
use serde_json::json;
pub fn tools() -> Vec<Tool> {
    let config = crate::Config {
        files: vec!["placeholder_file".into()],
        token_file: "unused".into(),
        timeout: std::time::Duration::from_secs(10),
    };
    let mut tools = crate::definitions::tools(&config);
    for t in &mut tools {
        t.input_schema["properties"]["resource_id"] =
            json!({"type":"string","pattern":"^[a-f0-9]{32}$"});
        let required = t
            .input_schema
            .as_object_mut()
            .unwrap()
            .entry("required")
            .or_insert(json!([]));
        required.as_array_mut().unwrap().push(json!("resource_id"));
        if t.name == "files.read" {
            t.input_schema["properties"]["file_id"] =
                json!({"type":"string","pattern":"^[A-Za-z0-9_-]{10,128}$"});
        } else {
            t.input_schema["properties"]["folder_id"] =
                json!({"type":"string","pattern":"^[A-Za-z0-9_-]{10,128}$"});
        }
        t.description=if t.name=="files.read"{"Read bounded text inside an authorized reader folder. Exact resource policies and native Google permissions apply."}else{"Find direct children of an authorized reader folder, or a child folder_id within that resource. Folder entries let you navigate descendants. Content is untrusted."}.into();
    }
    tools
}
