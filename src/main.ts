import {
  errorMessage,
  layoutDiagram,
  loadSample,
  parseDbml,
  parseMermaid,
  toDbml,
  toMermaid,
  validateDiagram,
} from "./lib/api";
import { DiagramRenderer } from "./canvas/renderer";
import type { CodeFormat, Diagram, ExportFormat } from "./types";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const els = {
  name: $("diagram-name"),
  entityList: $("entity-list") as HTMLElement as unknown as HTMLUListElement,
  entityCount: $("entity-count"),
  status: $("status-line"),
  code: $("code-editor") as HTMLTextAreaElement,
  codeFormat: $("code-format") as HTMLSelectElement,
  exportFormat: $("export-format") as HTMLSelectElement,
  fileInput: $("file-input") as HTMLInputElement,
  svg: $("diagram-svg") as unknown as SVGSVGElement,
};

let diagram: Diagram | null = null;
let selectedEntity: string | null = null;
let syncingCode = false;

const renderer = new DiagramRenderer(els.svg, {
  onSelectEntity: (name) => {
    selectedEntity = name;
    renderEntityList();
    renderer.setSelection(name);
  },
  onMoveEntity: (name, x, y) => {
    if (!diagram) return;
    const entity = diagram.entities.find((e) => e.name === name);
    if (!entity) return;
    entity.position = { x, y };
    renderer.render(diagram);
  },
});

function setStatus(msg: string, isError = false) {
  els.status.textContent = msg;
  els.status.classList.toggle("error", isError);
}

function renderEntityList() {
  els.entityList.innerHTML = "";
  if (!diagram) {
    els.entityCount.textContent = "0";
    return;
  }
  els.entityCount.textContent = String(diagram.entities.length);
  for (const entity of diagram.entities) {
    const li = document.createElement("li");
    li.textContent = entity.name;
    li.className = selectedEntity === entity.name ? "active" : "";
    li.title = `${entity.attributes.length} attributes`;
    li.addEventListener("click", () => {
      selectedEntity = entity.name;
      renderer.setSelection(entity.name);
      renderEntityList();
    });
    els.entityList.appendChild(li);
  }
}

async function setDiagram(next: Diagram, opts?: { fit?: boolean; syncCode?: boolean }) {
  diagram = next;
  els.name.textContent = next.name || "Untitled";
  renderEntityList();
  renderer.render(next);
  if (opts?.fit !== false) {
    requestAnimationFrame(() => renderer.fitToContent());
  }
  if (opts?.syncCode !== false) {
    await syncCodeFromDiagram();
  }
  const rels = next.relationships.length;
  setStatus(`${next.entities.length} entities · ${rels} relationships`);
}

async function syncCodeFromDiagram() {
  if (!diagram) return;
  syncingCode = true;
  try {
    const format = els.codeFormat.value as CodeFormat;
    els.code.value =
      format === "dbml" ? await toDbml(diagram) : await toMermaid(diagram);
  } catch (err) {
    setStatus(errorMessage(err), true);
  } finally {
    syncingCode = false;
  }
}

async function applyCode() {
  const source = els.code.value.trim();
  if (!source) {
    setStatus("Code panel is empty", true);
    return;
  }
  try {
    const format = els.codeFormat.value as CodeFormat;
    // Preserve positions by name when re-applying.
    const prev = diagram;
    let next =
      format === "dbml" ? await parseDbml(source) : await parseMermaid(source);
    if (prev) {
      for (const entity of next.entities) {
        const old = prev.entities.find(
          (e) => e.name.toLowerCase() === entity.name.toLowerCase(),
        );
        if (old?.position) entity.position = old.position;
      }
      if (!next.name || next.name.startsWith("Imported")) {
        next.name = prev.name;
      }
    }
    // Layout only entities still missing positions.
    next = await layoutDiagram(next, false);
    await setDiagram(next, { fit: !prev, syncCode: false });
    setStatus("Code applied");
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

async function onSample() {
  try {
    const next = await loadSample();
    await setDiagram(next);
    setStatus("Loaded MOHG HMS sample");
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

async function onLayout() {
  if (!diagram) return;
  try {
    const next = await layoutDiagram(diagram, true);
    await setDiagram(next, { syncCode: false });
    setStatus("Auto-layout applied");
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

async function onValidate() {
  if (!diagram) return;
  try {
    const report = await validateDiagram(diagram);
    if (report.ok) {
      const warn =
        report.warnings.length > 0
          ? ` with ${report.warnings.length} warning(s)`
          : "";
      setStatus(`Validation passed${warn}`);
      if (report.warnings.length) {
        console.info("Validation warnings:", report.warnings);
      }
    } else {
      setStatus(`Validation failed: ${report.errors[0]}`, true);
      console.error(report);
    }
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

function downloadText(filename: string, content: string, mime: string) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

async function exportContent(format: ExportFormat): Promise<{ name: string; content: string; mime: string }> {
  if (!diagram) throw new Error("No diagram loaded");
  const base = (diagram.name || "diagram").replace(/\s+/g, "_").toLowerCase();
  if (format === "json") {
    return {
      name: `${base}.er.json`,
      content: JSON.stringify(diagram, null, 2),
      mime: "application/json",
    };
  }
  if (format === "dbml") {
    return {
      name: `${base}.dbml`,
      content: await toDbml(diagram),
      mime: "text/plain",
    };
  }
  return {
    name: `${base}.mmd`,
    content: await toMermaid(diagram),
    mime: "text/plain",
  };
}

async function onExport() {
  try {
    const format = els.exportFormat.value as ExportFormat;
    const file = await exportContent(format);
    downloadText(file.name, file.content, file.mime);
    setStatus(`Exported ${file.name}`);
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

async function onCopy() {
  try {
    const format = els.exportFormat.value as ExportFormat;
    const file = await exportContent(format);
    await navigator.clipboard.writeText(file.content);
    setStatus(`Copied ${format.toUpperCase()} to clipboard`);
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

function onImportFile() {
  els.fileInput.click();
}

async function handleFile(file: File) {
  const text = await file.text();
  const name = file.name.toLowerCase();
  try {
    let next: Diagram;
    if (name.endsWith(".json")) {
      next = JSON.parse(text) as Diagram;
      next = await layoutDiagram(next, false);
    } else if (name.endsWith(".dbml") || text.includes("Table ") && text.includes("{")) {
      els.codeFormat.value = "dbml";
      next = await parseDbml(text);
    } else {
      els.codeFormat.value = "mermaid";
      next = await parseMermaid(text);
    }
    if (!next.name || next.name.startsWith("Imported")) {
      next.name = file.name.replace(/\.[^.]+$/, "");
    }
    els.code.value = text;
    await setDiagram(next, { syncCode: false });
    setStatus(`Imported ${file.name}`);
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

function bind() {
  $("btn-sample").addEventListener("click", () => void onSample());
  $("btn-apply-code").addEventListener("click", () => void applyCode());
  $("btn-layout").addEventListener("click", () => void onLayout());
  $("btn-validate").addEventListener("click", () => void onValidate());
  $("btn-export").addEventListener("click", () => void onExport());
  $("btn-copy").addEventListener("click", () => void onCopy());
  $("btn-import-file").addEventListener("click", onImportFile);
  els.fileInput.addEventListener("change", () => {
    const file = els.fileInput.files?.[0];
    if (file) void handleFile(file);
    els.fileInput.value = "";
  });
  els.codeFormat.addEventListener("change", () => {
    if (!syncingCode) void syncCodeFromDiagram();
  });
  els.code.addEventListener("keydown", (ev) => {
    if ((ev.metaKey || ev.ctrlKey) && ev.key === "Enter") {
      ev.preventDefault();
      void applyCode();
    }
  });

  // Drag-drop import on canvas / code
  for (const target of [document.body]) {
    target.addEventListener("dragover", (ev) => {
      ev.preventDefault();
    });
    target.addEventListener("drop", (ev) => {
      ev.preventDefault();
      const file = ev.dataTransfer?.files?.[0];
      if (file) void handleFile(file);
    });
  }
}

async function boot() {
  bind();
  try {
    await onSample();
  } catch {
    setStatus("Ready — paste Mermaid/DBML and Apply Code");
  }
}

void boot();
