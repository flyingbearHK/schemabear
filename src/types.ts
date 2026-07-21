export type Cardinality = "one" | "zero_or_one" | "zero_or_many" | "one_or_many";

export interface Position {
  x: number;
  y: number;
}

export interface Attribute {
  name: string;
  dataType: string;
  isPk: boolean;
  isFk: boolean;
  isUnique: boolean;
  isNullable: boolean;
  note?: string | null;
}

export interface Entity {
  id: string;
  name: string;
  attributes: Attribute[];
  position?: Position | null;
  note?: string | null;
}

export interface Relationship {
  id: string;
  fromEntity: string;
  toEntity: string;
  fromCardinality: Cardinality;
  toCardinality: Cardinality;
  label?: string | null;
  fromFields: string[];
  toFields: string[];
}

export interface Diagram {
  id: string;
  name: string;
  entities: Entity[];
  relationships: Relationship[];
  metadata?: {
    source?: string | null;
    notes?: string | null;
  } | null;
}

export interface ValidationReport {
  ok: boolean;
  errors: string[];
  warnings: string[];
}

export type CodeFormat = "mermaid" | "dbml";
export type ExportFormat = "mermaid" | "dbml" | "json";
