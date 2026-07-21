//! # er-core
//!
//! Portable ER diagram model with Mermaid and DBML interchange.
//!
//! Designed as a pure library so future backends (CLI, WASM, web API)
//! can reuse the same parse / validate / export pipeline.

pub mod dbml;
pub mod error;
pub mod layout;
pub mod mermaid;
pub mod model;
pub mod sample;
pub mod validate;

pub use dbml::{export_dbml, import_dbml};
pub use error::{Error, Result};
pub use layout::auto_layout;
pub use mermaid::{export_mermaid, import_mermaid};
pub use model::*;
pub use sample::{load_mohg_hms_sample, MOHG_HMS_SAMPLE_MERMAID};
pub use validate::{validate, ValidationReport};

/// Library version (mirrors crate version).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
