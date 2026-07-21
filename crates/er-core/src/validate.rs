use crate::model::Diagram;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Structural validation for a diagram.
pub fn validate(diagram: &Diagram) -> ValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if diagram.entities.is_empty() {
        errors.push("diagram has no entities".into());
    }

    let mut names = std::collections::HashSet::new();
    for entity in &diagram.entities {
        if entity.name.trim().is_empty() {
            errors.push(format!("entity {} has empty name", entity.id));
            continue;
        }
        let key = entity.name.to_ascii_lowercase();
        if !names.insert(key) {
            errors.push(format!("duplicate entity name: {}", entity.name));
        }

        let mut attr_names = std::collections::HashSet::new();
        let mut pk_count = 0usize;
        for attr in &entity.attributes {
            if attr.name.trim().is_empty() {
                errors.push(format!(
                    "entity {} has attribute with empty name",
                    entity.name
                ));
            }
            let ak = attr.name.to_ascii_lowercase();
            if !attr_names.insert(ak) {
                errors.push(format!("duplicate attribute {}.{}", entity.name, attr.name));
            }
            if attr.is_pk {
                pk_count += 1;
            }
        }
        if !entity.attributes.is_empty() && pk_count == 0 {
            warnings.push(format!("entity {} has no primary key", entity.name));
        }
    }

    for rel in &diagram.relationships {
        if diagram.entity_by_name(&rel.from_entity).is_none() {
            errors.push(format!(
                "relationship {} references unknown entity {}",
                rel.id, rel.from_entity
            ));
        }
        if diagram.entity_by_name(&rel.to_entity).is_none() {
            errors.push(format!(
                "relationship {} references unknown entity {}",
                rel.id, rel.to_entity
            ));
        }
        if rel.from_entity.eq_ignore_ascii_case(&rel.to_entity) {
            warnings.push(format!(
                "self-referential relationship on {}",
                rel.from_entity
            ));
        }
    }

    ValidationReport {
        ok: errors.is_empty(),
        errors,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Cardinality, Entity, Relationship};

    #[test]
    fn catches_unknown_entity() {
        let mut d = Diagram::new("t");
        d.entities.push(Entity::new("A"));
        d.relationships.push(Relationship::new(
            "A",
            "B",
            Cardinality::One,
            Cardinality::ZeroOrMany,
        ));
        let report = validate(&d);
        assert!(!report.ok);
        assert!(report.errors.iter().any(|e| e.contains("unknown entity")));
    }
}
