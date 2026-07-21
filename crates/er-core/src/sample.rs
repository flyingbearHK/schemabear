//! Built-in sample diagrams.

use crate::error::Result;
use crate::layout::auto_layout;
use crate::mermaid::import_mermaid;
use crate::model::Diagram;

/// Infor HMS–inspired hospitality sample (illustrative, not a production schema).
pub const INFOR_HMS_SAMPLE_MERMAID: &str =
    include_str!("../../../fixtures/infor_hms_sample.mmd");

pub fn load_infor_hms_sample() -> Result<Diagram> {
    let mut diagram = import_mermaid(INFOR_HMS_SAMPLE_MERMAID)?;
    diagram.name = "Infor HMS Sample".into();
    auto_layout(&mut diagram, true);
    Ok(diagram)
}
