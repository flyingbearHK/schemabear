import { invoke } from "@tauri-apps/api/core";
import type { Diagram, ValidationReport } from "../types";

export async function getVersion(): Promise<string> {
  return invoke<string>("get_version");
}

export async function loadSample(): Promise<Diagram> {
  return invoke<Diagram>("load_sample");
}

export async function parseMermaid(source: string): Promise<Diagram> {
  return invoke<Diagram>("parse_mermaid", { source });
}

export async function parseDbml(source: string): Promise<Diagram> {
  return invoke<Diagram>("parse_dbml", { source });
}

export async function toMermaid(diagram: Diagram): Promise<string> {
  return invoke<string>("to_mermaid", { diagram });
}

export async function toDbml(diagram: Diagram): Promise<string> {
  return invoke<string>("to_dbml", { diagram });
}

export async function layoutDiagram(diagram: Diagram, force = true): Promise<Diagram> {
  return invoke<Diagram>("layout_diagram", { diagram, force });
}

export async function validateDiagram(diagram: Diagram): Promise<ValidationReport> {
  return invoke<ValidationReport>("validate_diagram", { diagram });
}

export function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err && typeof err === "object") {
    const e = err as { message?: string; error?: string };
    if (e.message) return e.message;
    if (e.error) return e.error;
  }
  return String(err);
}
