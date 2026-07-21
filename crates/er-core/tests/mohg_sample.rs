use er_core::{auto_layout, export_dbml, export_mermaid, import_dbml, import_mermaid, validate};
use std::fs;
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/mohg_hms_sample.mmd")
}

#[test]
fn mohg_sample_mermaid_import_validate_layout_export() {
    let src = fs::read_to_string(fixture_path()).expect("read fixture");
    let mut diagram = import_mermaid(&src).expect("import mermaid");

    assert!(
        diagram.entities.len() >= 8,
        "expected hospitality entities, got {}",
        diagram.entities.len()
    );
    assert!(
        diagram.relationships.len() >= 8,
        "expected relationships, got {}",
        diagram.relationships.len()
    );

    let report = validate(&diagram);
    assert!(report.ok, "validation errors: {:?}", report.errors);

    auto_layout(&mut diagram, true);
    assert!(diagram.entities.iter().all(|e| e.position.is_some()));

    let mermaid = export_mermaid(&diagram);
    assert!(mermaid.contains("erDiagram"));
    assert!(mermaid.contains("RESERVATION"));

    let dbml = export_dbml(&diagram);
    assert!(dbml.contains("Table "));
    assert!(dbml.contains("Ref:"));

    // Round-trip DBML keeps tables.
    let reimport = import_dbml(&dbml).expect("import dbml");
    assert_eq!(reimport.entities.len(), diagram.entities.len());

    // Round-trip Mermaid keeps entity count.
    let m2 = import_mermaid(&mermaid).expect("reimport mermaid");
    assert_eq!(m2.entities.len(), diagram.entities.len());
}
