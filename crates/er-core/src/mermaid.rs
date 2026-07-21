//! Mermaid `erDiagram` import/export.
//!
//! Supports the common subset used by AI generators and mermaid.live:
//! - relationship lines: `A ||--o{ B : label`
//! - entity attribute blocks
//! - attribute markers: PK, FK, UK

use crate::error::{Error, Result};
use crate::model::{Attribute, Cardinality, Diagram, DiagramMetadata, Position, Relationship};
use uuid::Uuid;

/// Parse a Mermaid `erDiagram` document into a [`Diagram`].
pub fn import_mermaid(input: &str) -> Result<Diagram> {
    let mut diagram = Diagram::new("Imported Diagram");
    diagram.metadata = Some(DiagramMetadata {
        source: Some("mermaid".into()),
        notes: None,
    });

    let mut in_er = false;
    let mut current_entity: Option<String> = None;
    let mut line_no = 0usize;

    for raw in input.lines() {
        line_no += 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }

        let lower = line.to_ascii_lowercase();
        if lower.starts_with("erdiagram") {
            in_er = true;
            let rest = line["erDiagram".len()..].trim();
            if !rest.is_empty() {
                diagram.name = rest.to_string();
            }
            continue;
        }

        // Allow diagrams that omit the header (AI often pastes body only).
        if !in_er {
            if looks_like_relationship(line) || looks_like_entity_open(line) {
                in_er = true;
            } else if lower.starts_with("graph")
                || lower.starts_with("flowchart")
                || lower.starts_with("sequencediagram")
            {
                return Err(Error::parse(
                    line_no,
                    "expected erDiagram, found a different Mermaid diagram type",
                ));
            } else {
                continue;
            }
        }

        if let Some(name) = current_entity.clone() {
            if line == "}" {
                current_entity = None;
                continue;
            }
            let attr = parse_attribute(line).map_err(|m| Error::parse(line_no, m))?;
            if let Some(entity) = diagram.entity_by_name_mut(&name) {
                // Replace if same name already present (last wins).
                if let Some(idx) = entity
                    .attributes
                    .iter()
                    .position(|a| a.name.eq_ignore_ascii_case(&attr.name))
                {
                    entity.attributes[idx] = attr;
                } else {
                    entity.attributes.push(attr);
                }
            }
            continue;
        }

        if let Some(name) = parse_entity_open(line) {
            diagram.ensure_entity(&name);
            current_entity = Some(name);
            continue;
        }

        if let Some(rel) = parse_relationship(line).map_err(|m| Error::parse(line_no, m))? {
            diagram.ensure_entity(&rel.from_entity);
            diagram.ensure_entity(&rel.to_entity);
            diagram.relationships.push(rel);
            continue;
        }

        // Bare entity name line (no block).
        if is_ident(line) {
            diagram.ensure_entity(line);
            continue;
        }

        return Err(Error::parse(
            line_no,
            format!("unrecognized Mermaid ER syntax: {line}"),
        ));
    }

    if current_entity.is_some() {
        return Err(Error::parse(line_no, "unclosed entity block"));
    }

    if diagram.entities.is_empty() && diagram.relationships.is_empty() {
        return Err(Error::parse(
            1,
            "no entities or relationships found in Mermaid input",
        ));
    }

    Ok(diagram)
}

/// Serialize a diagram to Mermaid `erDiagram` text.
pub fn export_mermaid(diagram: &Diagram) -> String {
    let mut out = String::from("erDiagram\n");

    for rel in &diagram.relationships {
        let left = rel.from_cardinality.as_mermaid_token();
        // Mermaid uses mirrored tokens on the right side.
        let right = match rel.to_cardinality {
            Cardinality::One => "||",
            Cardinality::ZeroOrOne => "o|",
            Cardinality::ZeroOrMany => "o{",
            Cardinality::OneOrMany => "|{",
        };
        let label = rel
            .label
            .as_deref()
            .map(|l| sanitize_label(l))
            .unwrap_or_else(|| "relates".into());
        out.push_str(&format!(
            "    {} {}--{} {} : {}\n",
            rel.from_entity, left, right, rel.to_entity, label
        ));
    }

    if !diagram.relationships.is_empty() {
        out.push('\n');
    }

    for entity in &diagram.entities {
        if entity.attributes.is_empty() {
            out.push_str(&format!("    {}\n", entity.name));
            continue;
        }
        out.push_str(&format!("    {} {{\n", entity.name));
        for attr in &entity.attributes {
            let mut markers = Vec::new();
            if attr.is_pk {
                markers.push("PK");
            }
            if attr.is_fk {
                markers.push("FK");
            }
            if attr.is_unique && !attr.is_pk {
                markers.push("UK");
            }
            let marker = if markers.is_empty() {
                String::new()
            } else {
                format!(" {}", markers.join(","))
            };
            let ty = if attr.data_type.is_empty() {
                "string"
            } else {
                &attr.data_type
            };
            // Mermaid attribute: type name PK,FK
            out.push_str(&format!("        {} {}{}\n", ty, attr.name, marker));
        }
        out.push_str("    }\n");
    }

    out
}

fn strip_comment(line: &str) -> &str {
    if let Some(idx) = line.find("%%") {
        &line[..idx]
    } else {
        line
    }
}

fn looks_like_relationship(line: &str) -> bool {
    line.contains("--") && line.split_whitespace().count() >= 3
}

fn looks_like_entity_open(line: &str) -> bool {
    parse_entity_open(line).is_some()
}

fn parse_entity_open(line: &str) -> Option<String> {
    let line = line.trim();
    if let Some(rest) = line.strip_suffix('{') {
        let name = rest.trim();
        if is_ident(name) {
            return Some(name.to_string());
        }
    }
    None
}

fn parse_attribute(line: &str) -> std::result::Result<Attribute, String> {
    // Formats:
    //   type name
    //   type name PK
    //   type name PK, FK
    //   name type          (less common; try type-first first)
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 2 {
        return Err(format!("attribute needs type and name: {line}"));
    }

    let mut is_pk = false;
    let mut is_fk = false;
    let mut is_unique = false;
    let mut end = tokens.len();

    // Consume trailing markers.
    while end > 2 {
        let t = tokens[end - 1].trim_matches(',');
        let upper = t.to_ascii_uppercase();
        if matches!(
            upper.as_str(),
            "PK" | "FK" | "UK" | "UNIQUE" | "NN" | "NOT_NULL"
        ) {
            match upper.as_str() {
                "PK" => is_pk = true,
                "FK" => is_fk = true,
                "UK" | "UNIQUE" => is_unique = true,
                "NN" | "NOT_NULL" => {}
                _ => {}
            }
            end -= 1;
        } else if tokens[end - 1].contains(',') {
            for part in tokens[end - 1].split(',') {
                match part.trim().to_ascii_uppercase().as_str() {
                    "PK" => is_pk = true,
                    "FK" => is_fk = true,
                    "UK" | "UNIQUE" => is_unique = true,
                    "" => {}
                    other => return Err(format!("unknown attribute marker: {other}")),
                }
            }
            end -= 1;
        } else {
            break;
        }
    }

    if end < 2 {
        return Err(format!("attribute needs type and name: {line}"));
    }

    // type name [markers]
    let data_type = tokens[0].to_string();
    let name = tokens[1..end].join("_");
    if !is_ident(&name) && !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("invalid attribute name: {name}"));
    }

    let mut attr = Attribute::new(name, data_type);
    attr.is_pk = is_pk;
    attr.is_fk = is_fk;
    attr.is_unique = is_unique;
    if is_pk {
        attr.is_nullable = false;
    }
    Ok(attr)
}

fn parse_relationship(line: &str) -> std::result::Result<Option<Relationship>, String> {
    // A ||--o{ B : label
    // A ||--o{ B : "has many"
    if !line.contains("--") {
        return Ok(None);
    }

    let (left_part, right_part) = line
        .split_once(':')
        .map(|(l, r)| (l.trim(), Some(r.trim())))
        .unwrap_or((line.trim(), None));

    let tokens: Vec<&str> = left_part.split_whitespace().collect();
    if tokens.len() < 3 {
        return Ok(None);
    }

    // Find the connector token containing "--"
    let conn_idx = tokens
        .iter()
        .position(|t| t.contains("--"))
        .ok_or_else(|| format!("missing relationship connector: {line}"))?;

    if conn_idx == 0 || conn_idx + 1 >= tokens.len() {
        return Err(format!("invalid relationship: {line}"));
    }

    let from = tokens[..conn_idx].join("_");
    let to = tokens[conn_idx + 1..].join("_");
    if from.is_empty() || to.is_empty() {
        return Err(format!("invalid relationship endpoints: {line}"));
    }

    let conn = tokens[conn_idx];
    let (left_card, right_card) = parse_connector(conn)?;

    let label = right_part.map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string());

    let mut rel = Relationship::new(from, to, left_card, right_card);
    rel.id = Uuid::new_v4().to_string();
    rel.label = label.filter(|s| !s.is_empty());
    Ok(Some(rel))
}

fn parse_connector(conn: &str) -> std::result::Result<(Cardinality, Cardinality), String> {
    // Forms: ||--o{   }o--||   |o--||   ||--||  etc.
    let parts: Vec<&str> = conn.split("--").collect();
    if parts.len() != 2 {
        return Err(format!("invalid connector: {conn}"));
    }
    let left = normalize_card_token(parts[0], true)?;
    let right = normalize_card_token(parts[1], false)?;
    Ok((left, right))
}

fn normalize_card_token(token: &str, is_left: bool) -> std::result::Result<Cardinality, String> {
    let t = token.trim();
    // Accept both orientations used by Mermaid.
    match t {
        "||" => Ok(Cardinality::One),
        "|o" | "o|" => Ok(Cardinality::ZeroOrOne),
        "}o" | "o{" => Ok(Cardinality::ZeroOrMany),
        "}|" | "|{" => Ok(Cardinality::OneOrMany),
        // Sometimes people write only one side loosely
        "|" if is_left => Ok(Cardinality::One),
        _ => {
            // Try Cardinality helper
            Cardinality::from_mermaid_token(t)
                .ok_or_else(|| format!("unknown cardinality token '{t}'"))
        }
    }
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        }
        _ => false,
    }
}

fn sanitize_label(label: &str) -> String {
    let cleaned: String = label
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if cleaned.is_empty() {
        "relates".into()
    } else {
        cleaned
    }
}

/// Apply positions from a previous diagram onto a freshly imported one (by entity name).
pub fn merge_positions(target: &mut Diagram, source: &Diagram) {
    for entity in &mut target.entities {
        if let Some(prev) = source.entity_by_name(&entity.name) {
            if entity.position.is_none() {
                entity.position = prev.position;
            }
        }
    }
    // Preserve name if target still has default-ish name.
    if target.name == "Imported Diagram" && source.name != "Imported Diagram" {
        target.name = source.name.clone();
    }
}

/// Restore entity positions after re-import when IDs changed.
pub fn preserve_layout(old: &Diagram, new_diagram: &mut Diagram) {
    merge_positions(new_diagram, old);
    // Keep explicit positions list stable for UI.
    let _ = Position { x: 0.0, y: 0.0 };
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_simple_er() {
        let src = r#"
erDiagram
    CUSTOMER ||--o{ ORDER : places
    CUSTOMER {
        string name
        string custNumber PK
    }
    ORDER {
        int orderNumber PK
        string deliveryAddress
    }
"#;
        let d = import_mermaid(src).unwrap();
        assert_eq!(d.entities.len(), 2);
        assert_eq!(d.relationships.len(), 1);
        let customer = d.entity_by_name("CUSTOMER").unwrap();
        assert!(customer.attributes.iter().any(|a| a.is_pk));
        assert_eq!(d.relationships[0].from_cardinality, Cardinality::One);
        assert_eq!(d.relationships[0].to_cardinality, Cardinality::ZeroOrMany);
    }

    #[test]
    fn round_trip_mermaid() {
        let src = r#"
erDiagram
    PROPERTY ||--o{ RESERVATION : hosts
    PROPERTY {
        string property_code PK
        string name
    }
    RESERVATION {
        string reservation_id PK
        string property_code FK
        date arrival_date
    }
"#;
        let d = import_mermaid(src).unwrap();
        let out = export_mermaid(&d);
        let d2 = import_mermaid(&out).unwrap();
        assert_eq!(d2.entities.len(), d.entities.len());
        assert_eq!(d2.relationships.len(), d.relationships.len());
    }
}
