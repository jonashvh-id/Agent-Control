use crate::Config;
use control_core::connector::Tool;
pub fn tools(config: &Config) -> Vec<Tool> {
    let mut tools: Vec<Tool> =
        serde_json::from_str(include_str!("tools.json")).expect("embedded tool schemas");
    for tool in &mut tools {
        tool.input_schema["properties"]["repository"]["enum"] =
            serde_json::json!(config.repositories);
    }
    tools
}
