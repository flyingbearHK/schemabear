use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Top-level ER diagram document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Diagram {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<DiagramMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiagramMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub attributes: Vec<Attribute>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attribute {
    pub name: String,
    #[serde(default = "default_type")]
    pub data_type: String,
    #[serde(default)]
    pub is_pk: bool,
    #[serde(default)]
    pub is_fk: bool,
    #[serde(default)]
    pub is_unique: bool,
    #[serde(default = "default_true")]
    pub is_nullable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

fn default_type() -> String {
    "string".into()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Relationship cardinality at one end of an association.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Cardinality {
    #[default]
    One,
    ZeroOrOne,
    ZeroOrMany,
    OneOrMany,
}

impl Cardinality {
    pub fn as_mermaid_token(self) -> &'static str {
        match self {
            Self::One => "||",
            Self::ZeroOrOne => "|o",
            Self::ZeroOrMany => "o{",
            Self::OneOrMany => "|{",
        }
    }

    pub fn from_mermaid_token(token: &str) -> Option<Self> {
        match token {
            "||" => Some(Self::One),
            "|o" | "o|" => Some(Self::ZeroOrOne),
            "}o" | "o{" => Some(Self::ZeroOrMany),
            "}| " | "}|" | "|{" => Some(Self::OneOrMany),
            _ => None,
        }
    }

    /// DBML-ish multiplicity label.
    pub fn as_label(self) -> &'static str {
        match self {
            Self::One => "1",
            Self::ZeroOrOne => "0..1",
            Self::ZeroOrMany => "0..*",
            Self::OneOrMany => "1..*",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    pub id: String,
    pub from_entity: String,
    pub to_entity: String,
    pub from_cardinality: Cardinality,
    pub to_cardinality: Cardinality,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default)]
    pub from_fields: Vec<String>,
    #[serde(default)]
    pub to_fields: Vec<String>,
}

impl Diagram {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            entities: Vec::new(),
            relationships: Vec::new(),
            metadata: None,
        }
    }

    pub fn entity_by_name(&self, name: &str) -> Option<&Entity> {
        self.entities
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
    }

    pub fn entity_by_name_mut(&mut self, name: &str) -> Option<&mut Entity> {
        self.entities
            .iter_mut()
            .find(|e| e.name.eq_ignore_ascii_case(name))
    }

    pub fn ensure_entity(&mut self, name: &str) -> &mut Entity {
        if let Some(idx) = self
            .entities
            .iter()
            .position(|e| e.name.eq_ignore_ascii_case(name))
        {
            return &mut self.entities[idx];
        }
        self.entities.push(Entity {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            attributes: Vec::new(),
            position: None,
            note: None,
        });
        self.entities.last_mut().expect("just pushed")
    }
}

impl Entity {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            attributes: Vec::new(),
            position: None,
            note: None,
        }
    }
}

impl Attribute {
    pub fn new(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            is_pk: false,
            is_fk: false,
            is_unique: false,
            is_nullable: true,
            note: None,
        }
    }

    pub fn pk(mut self) -> Self {
        self.is_pk = true;
        self.is_nullable = false;
        self
    }

    pub fn fk(mut self) -> Self {
        self.is_fk = true;
        self
    }

    pub fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }

    pub fn not_null(mut self) -> Self {
        self.is_nullable = false;
        self
    }
}

impl Relationship {
    pub fn new(
        from_entity: impl Into<String>,
        to_entity: impl Into<String>,
        from_cardinality: Cardinality,
        to_cardinality: Cardinality,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from_entity: from_entity.into(),
            to_entity: to_entity.into(),
            from_cardinality,
            to_cardinality,
            label: None,
            from_fields: Vec::new(),
            to_fields: Vec::new(),
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}
