//! Built-in sample diagrams.

use crate::error::Result;
use crate::layout::auto_layout;
use crate::mermaid::import_mermaid;
use crate::model::Diagram;

/// MOHG / Infor HMS inspired sample (illustrative).
pub const MOHG_HMS_SAMPLE_MERMAID: &str = include_str!("../../../fixtures/mohg_hms_sample.mmd");

pub fn load_mohg_hms_sample() -> Result<Diagram> {
    let mut diagram = import_mermaid(MOHG_HMS_SAMPLE_MERMAID)?;
    diagram.name = "MOHG HMS Sample".into();
    auto_layout(&mut diagram, true);
    Ok(diagram)
}
