use er_core::{
    auto_layout, auto_layout_with, export_dbml, export_mermaid, import_dbml, import_mermaid,
    load_infor_hms_sample, validate, Diagram, LayoutOptions, ValidationReport, VERSION,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub message: String,
}

impl From<er_core::Error> for CommandError {
    fn from(value: er_core::Error) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

#[tauri::command]
pub fn get_version() -> String {
    VERSION.to_string()
}

#[tauri::command]
pub fn load_sample() -> Result<Diagram, CommandError> {
    Ok(load_infor_hms_sample()?)
}

#[tauri::command]
pub fn parse_mermaid(source: String) -> Result<Diagram, CommandError> {
    let mut diagram = import_mermaid(&source)?;
    auto_layout(&mut diagram, false);
    Ok(diagram)
}

#[tauri::command]
pub fn parse_dbml(source: String) -> Result<Diagram, CommandError> {
    let mut diagram = import_dbml(&source)?;
    auto_layout(&mut diagram, false);
    Ok(diagram)
}

#[tauri::command]
pub fn to_mermaid(diagram: Diagram) -> Result<String, CommandError> {
    Ok(export_mermaid(&diagram))
}

#[tauri::command]
pub fn to_dbml(diagram: Diagram) -> Result<String, CommandError> {
    Ok(export_dbml(&diagram))
}

#[tauri::command]
pub fn layout_diagram(
    mut diagram: Diagram,
    force: bool,
    options: Option<LayoutOptions>,
) -> Result<Diagram, CommandError> {
    let mut opts = options.unwrap_or_default();
    opts.force = force;
    auto_layout_with(&mut diagram, opts);
    Ok(diagram)
}

#[tauri::command]
pub fn validate_diagram(diagram: Diagram) -> Result<ValidationReport, CommandError> {
    Ok(validate(&diagram))
}
