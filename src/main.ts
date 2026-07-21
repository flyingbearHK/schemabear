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
import type {
  Attribute,
  Cardinality,
  CodeFormat,
  Diagram,
  Entity,
  ExportFormat,
  Relationship,
} from "./types";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const els = {
  name: $("diagram-name"),
  entityList: $("entity-list") as unknown as HTMLUListElement,
  entityCount: $("entity-count"),
  status: $("status-line"),
  code: $("code-editor") as HTMLTextAreaElement,
  codeFormat: $("code-format") as HTMLSelectElement,
  exportFormat: $("export-format") as HTMLSelectElement,
  fileInput: $("file-input") as HTMLInputElement,
  svg: $("diagram-svg") as unknown as SVGSVGElement,
  zoomLabel: $("btn-zoom-label"),
  editEmpty: $("edit-empty"),
  editEntity: $("edit-entity"),
  fieldEntityName: $("field-entity-name") as HTMLInputElement,
  attrList: $("attr-list"),
  relList: $("rel-list"),
  relFrom: $("rel-from") as HTMLSelectElement,
  relTo: $("rel-to") as HTMLSelectElement,
  relFromCard: $("rel-from-card") as HTMLSelectElement,
  relToCard: $("rel-to-card") as HTMLSelectElement,
  relLabel: $("rel-label") as HTMLInputElement,
};

let diagram: Diagram | null = null;
let selectedEntity: string | null = null;
let syncingCode = false;
let syncTimer: number | null = null;
let suppressNameInput = false;

const renderer = new DiagramRenderer(els.svg, {
  onSelectEntity: (name) => {
    selectedEntity = name;
    renderEntityList();
    // setSelection is already applied inside the renderer for pointer paths;
    // keep in sync for sidebar-driven selection.
    renderer.setSelection(name);
    renderEditor();
  },
  onMoveEntity: (name, x, y) => {
    // Model-only update. The renderer already moved the card + edges on the
    // fast path — do NOT call renderer.render() here (that was the lag source).
    if (!diagram) return;
    const entity = diagram.entities.find((e) => e.name === name);
    if (!entity) return;
    entity.position = { x, y };
  },
  onZoomChange: (k) => updateZoomLabel(k),
});

function uid(): string {
  return crypto.randomUUID();
}

function setStatus(msg: string, isError = false) {
  els.status.textContent = msg;
  els.status.classList.toggle("error", isError);
}

function updateZoomLabel(k?: number) {
  const z = k ?? renderer.getZoom();
  els.zoomLabel.textContent = `${Math.round(z * 100)}%`;
}

function scheduleCodeSync() {
  if (syncTimer != null) window.clearTimeout(syncTimer);
  syncTimer = window.setTimeout(() => {
    void syncCodeFromDiagram();
  }, 280);
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
    li.addEventListener("click", () => selectEntity(entity.name));
    els.entityList.appendChild(li);
  }
}

function selectEntity(name: string | null) {
  selectedEntity = name;
  renderer.setSelection(name);
  renderEntityList();
  renderEditor();
  if (name) {
    // Ensure Edit tab is visible when selecting.
    activateTab("edit");
  }
}

function selected(): Entity | null {
  if (!diagram || !selectedEntity) return null;
  return diagram.entities.find((e) => e.name === selectedEntity) ?? null;
}

function cardLabel(c: Cardinality): string {
  switch (c) {
    case "one":
      return "1";
    case "zero_or_one":
      return "0..1";
    case "zero_or_many":
      return "0..*";
    case "one_or_many":
      return "1..*";
  }
}

function renderEditor() {
  const entity = selected();
  if (!entity || !diagram) {
    els.editEmpty.classList.remove("hidden");
    els.editEntity.classList.add("hidden");
    return;
  }
  els.editEmpty.classList.add("hidden");
  els.editEntity.classList.remove("hidden");

  suppressNameInput = true;
  els.fieldEntityName.value = entity.name;
  suppressNameInput = false;

  // Attributes
  els.attrList.innerHTML = "";
  entity.attributes.forEach((attr, idx) => {
    els.attrList.appendChild(buildAttrRow(entity, attr, idx));
  });

  // Relationships involving this entity
  els.relList.innerHTML = "";
  const rels = diagram.relationships.filter(
    (r) => r.fromEntity === entity.name || r.toEntity === entity.name,
  );
  if (rels.length === 0) {
    const empty = document.createElement("div");
    empty.className = "muted";
    empty.textContent = "No relationships yet.";
    els.relList.appendChild(empty);
  } else {
    for (const rel of rels) {
      els.relList.appendChild(buildRelItem(rel));
    }
  }

  // Populate relationship selects
  fillEntitySelect(els.relFrom, entity.name);
  fillEntitySelect(els.relTo, diagram.entities.find((e) => e.name !== entity.name)?.name ?? entity.name);
}

function fillEntitySelect(select: HTMLSelectElement, preferred?: string) {
  if (!diagram) return;
  const current = preferred ?? select.value;
  select.innerHTML = "";
  for (const e of diagram.entities) {
    const opt = document.createElement("option");
    opt.value = e.name;
    opt.textContent = e.name;
    select.appendChild(opt);
  }
  if (current && [...select.options].some((o) => o.value === current)) {
    select.value = current;
  }
}

function buildAttrRow(entity: Entity, attr: Attribute, idx: number): HTMLElement {
  const row = document.createElement("div");
  row.className = "attr-row";

  const name = document.createElement("input");
  name.type = "text";
  name.value = attr.name;
  name.placeholder = "name";
  name.spellcheck = false;
  name.addEventListener("change", () => {
    attr.name = name.value.trim() || attr.name;
    commitVisualEdit(`Updated attribute on ${entity.name}`);
  });

  const type = document.createElement("input");
  type.type = "text";
  type.value = attr.dataType;
  type.placeholder = "type";
  type.spellcheck = false;
  type.addEventListener("change", () => {
    attr.dataType = type.value.trim() || "string";
    commitVisualEdit(`Updated type on ${entity.name}.${attr.name}`);
  });

  const remove = document.createElement("button");
  remove.type = "button";
  remove.className = "remove";
  remove.textContent = "×";
  remove.title = "Remove attribute";
  remove.addEventListener("click", () => {
    entity.attributes.splice(idx, 1);
    commitVisualEdit(`Removed attribute from ${entity.name}`);
    renderEditor();
  });

  const flags = document.createElement("div");
  flags.className = "flags";
  flags.append(
    flagCheckbox("PK", attr.isPk, (v) => {
      attr.isPk = v;
      if (v) attr.isNullable = false;
      commitVisualEdit(`PK flag on ${entity.name}.${attr.name}`);
      renderEditor();
    }),
    flagCheckbox("FK", attr.isFk, (v) => {
      attr.isFk = v;
      commitVisualEdit(`FK flag on ${entity.name}.${attr.name}`);
    }),
    flagCheckbox("UK", attr.isUnique, (v) => {
      attr.isUnique = v;
      commitVisualEdit(`UK flag on ${entity.name}.${attr.name}`);
    }),
    flagCheckbox("Null", attr.isNullable, (v) => {
      attr.isNullable = v;
      commitVisualEdit(`Nullability on ${entity.name}.${attr.name}`);
    }),
  );

  row.append(name, type, remove, flags);
  return row;
}

function flagCheckbox(
  label: string,
  checked: boolean,
  onChange: (v: boolean) => void,
): HTMLLabelElement {
  const wrap = document.createElement("label");
  const input = document.createElement("input");
  input.type = "checkbox";
  input.checked = checked;
  input.addEventListener("change", () => onChange(input.checked));
  wrap.append(input, document.createTextNode(label));
  return wrap;
}

function buildRelItem(rel: Relationship): HTMLElement {
  const row = document.createElement("div");
  row.className = "rel-item";
  const body = document.createElement("div");
  body.innerHTML = `<div><strong>${rel.fromEntity}</strong> → <strong>${rel.toEntity}</strong></div>
    <div class="meta">${cardLabel(rel.fromCardinality)} … ${cardLabel(rel.toCardinality)}${rel.label ? ` · ${rel.label}` : ""}</div>`;
  const del = document.createElement("button");
  del.type = "button";
  del.className = "tiny danger";
  del.textContent = "Delete";
  del.addEventListener("click", () => {
    if (!diagram) return;
    diagram.relationships = diagram.relationships.filter((r) => r.id !== rel.id);
    commitVisualEdit("Deleted relationship");
    renderEditor();
  });
  row.append(body, del);
  return row;
}

function commitVisualEdit(message: string) {
  if (!diagram) return;
  // Keep selection stable if entity renamed via other means
  renderEntityList();
  renderer.render(diagram);
  renderer.setSelection(selectedEntity);
  scheduleCodeSync();
  setStatus(message);
}

async function setDiagram(next: Diagram, opts?: { fit?: boolean; syncCode?: boolean }) {
  diagram = next;
  els.name.textContent = next.name || "Untitled";
  // Drop selection if entity vanished
  if (selectedEntity && !next.entities.some((e) => e.name === selectedEntity)) {
    selectedEntity = null;
  }
  renderEntityList();
  renderer.render(next);
  renderer.setSelection(selectedEntity);
  renderEditor();
  if (opts?.fit !== false) {
    requestAnimationFrame(() => {
      renderer.fitToContent();
      updateZoomLabel();
    });
  } else {
    updateZoomLabel();
  }
  if (opts?.syncCode !== false) {
    await syncCodeFromDiagram();
  }
  setStatus(`${next.entities.length} entities · ${next.relationships.length} relationships`);
}

async function syncCodeFromDiagram() {
  if (!diagram) return;
  syncingCode = true;
  try {
    const format = els.codeFormat.value as CodeFormat;
    els.code.value = format === "dbml" ? await toDbml(diagram) : await toMermaid(diagram);
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
    const prev = diagram;
    let next = format === "dbml" ? await parseDbml(source) : await parseMermaid(source);
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
    next = await layoutDiagram(next, false);
    selectedEntity = null;
    await setDiagram(next, { fit: !prev, syncCode: false });
    setStatus("Code applied");
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

async function onSample() {
  try {
    const next = await loadSample();
    selectedEntity = null;
    await setDiagram(next);
    setStatus("Loaded MOHG HMS sample");
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

async function onLayout() {
  if (!diagram) return;
  try {
    setStatus("Arranging…");
    const next = await layoutDiagram(diagram, true);
    await setDiagram(next, { syncCode: false, fit: true });
    setStatus("Auto-arranged entities and relationships");
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

type ThemeMode = "system" | "light" | "dark";
const THEME_KEY = "er-diagram.theme";

function resolveTheme(mode: ThemeMode): "light" | "dark" {
  if (mode === "light" || mode === "dark") return mode;
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

function applyTheme(mode: ThemeMode) {
  const root = document.documentElement;
  root.dataset.theme = mode;
  // Help native form controls / scrollbars pick the right scheme.
  root.style.colorScheme = mode === "system" ? "light dark" : mode;
  localStorage.setItem(THEME_KEY, mode);
  const select = document.getElementById("theme-select") as HTMLSelectElement | null;
  if (select) select.value = mode;
  // Re-paint SVG so CSS variables on markers refresh if needed.
  if (diagram) renderer.render(diagram);
}

function initTheme() {
  const saved = localStorage.getItem(THEME_KEY) as ThemeMode | null;
  const mode: ThemeMode =
    saved === "light" || saved === "dark" || saved === "system" ? saved : "system";
  applyTheme(mode);

  const mq = window.matchMedia("(prefers-color-scheme: light)");
  const onOsChange = () => {
    const current = (localStorage.getItem(THEME_KEY) as ThemeMode | null) ?? "system";
    if (current === "system") applyTheme("system");
  };
  mq.addEventListener("change", onOsChange);
  void resolveTheme; // keep helper for future status text if needed
}

async function onValidate() {
  if (!diagram) return;
  try {
    const report = await validateDiagram(diagram);
    if (report.ok) {
      const warn =
        report.warnings.length > 0 ? ` with ${report.warnings.length} warning(s)` : "";
      setStatus(`Validation passed${warn}`);
    } else {
      setStatus(`Validation failed: ${report.errors[0]}`, true);
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

async function exportContent(
  format: ExportFormat,
): Promise<{ name: string; content: string; mime: string }> {
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
    return { name: `${base}.dbml`, content: await toDbml(diagram), mime: "text/plain" };
  }
  return { name: `${base}.mmd`, content: await toMermaid(diagram), mime: "text/plain" };
}

async function onExport() {
  try {
    const file = await exportContent(els.exportFormat.value as ExportFormat);
    downloadText(file.name, file.content, file.mime);
    setStatus(`Exported ${file.name}`);
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

async function onCopy() {
  try {
    const file = await exportContent(els.exportFormat.value as ExportFormat);
    await navigator.clipboard.writeText(file.content);
    setStatus(`Copied ${els.exportFormat.value.toUpperCase()} to clipboard`);
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
    } else if (name.endsWith(".dbml") || (text.includes("Table ") && text.includes("{"))) {
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
    selectedEntity = null;
    await setDiagram(next, { syncCode: false });
    setStatus(`Imported ${file.name}`);
  } catch (err) {
    setStatus(errorMessage(err), true);
  }
}

function ensureDiagram(): Diagram {
  if (!diagram) {
    diagram = {
      id: uid(),
      name: "Untitled",
      entities: [],
      relationships: [],
    };
  }
  return diagram;
}

function uniqueEntityName(base = "Entity"): string {
  const d = ensureDiagram();
  let n = 1;
  let name = base;
  const taken = new Set(d.entities.map((e) => e.name.toLowerCase()));
  while (taken.has(name.toLowerCase())) {
    n += 1;
    name = `${base}${n}`;
  }
  return name;
}

function addEntity() {
  const d = ensureDiagram();
  const name = uniqueEntityName("Entity");
  // Place near viewport center in world coords.
  const t = renderer.getTransform();
  const rect = els.svg.getBoundingClientRect();
  const wx = (rect.width / 2 - t.x) / t.k - 120;
  const wy = (rect.height / 2 - t.y) / t.k - 40;
  const entity: Entity = {
    id: uid(),
    name,
    attributes: [
      {
        name: "id",
        dataType: "string",
        isPk: true,
        isFk: false,
        isUnique: false,
        isNullable: false,
      },
    ],
    position: { x: wx, y: wy },
  };
  d.entities.push(entity);
  if (!d.name) d.name = "Untitled";
  selectEntity(name);
  commitVisualEdit(`Added ${name}`);
  renderer.render(d);
}

function renameSelectedEntity(nextName: string) {
  const entity = selected();
  if (!entity || !diagram) return;
  const name = nextName.trim();
  if (!name || name === entity.name) return;
  if (diagram.entities.some((e) => e !== entity && e.name.toLowerCase() === name.toLowerCase())) {
    setStatus(`Entity name already exists: ${name}`, true);
    els.fieldEntityName.value = entity.name;
    return;
  }
  const old = entity.name;
  entity.name = name;
  for (const rel of diagram.relationships) {
    if (rel.fromEntity === old) rel.fromEntity = name;
    if (rel.toEntity === old) rel.toEntity = name;
  }
  selectedEntity = name;
  commitVisualEdit(`Renamed ${old} → ${name}`);
  renderEditor();
}

function addAttribute() {
  const entity = selected();
  if (!entity) return;
  let n = entity.attributes.length + 1;
  let name = `field_${n}`;
  while (entity.attributes.some((a) => a.name === name)) {
    n += 1;
    name = `field_${n}`;
  }
  entity.attributes.push({
    name,
    dataType: "string",
    isPk: false,
    isFk: false,
    isUnique: false,
    isNullable: true,
  });
  commitVisualEdit(`Added attribute on ${entity.name}`);
  renderEditor();
}

function deleteSelectedEntity() {
  const entity = selected();
  if (!entity || !diagram) return;
  if (!confirm(`Delete entity ${entity.name} and its relationships?`)) return;
  diagram.entities = diagram.entities.filter((e) => e.id !== entity.id);
  diagram.relationships = diagram.relationships.filter(
    (r) => r.fromEntity !== entity.name && r.toEntity !== entity.name,
  );
  selectedEntity = null;
  commitVisualEdit(`Deleted ${entity.name}`);
  renderEditor();
}

function addRelationship() {
  if (!diagram) return;
  const from = els.relFrom.value;
  const to = els.relTo.value;
  if (!from || !to) {
    setStatus("Pick both relationship ends", true);
    return;
  }
  if (from === to) {
    setStatus("Self-relationships: use two different ends or allow later", true);
  }
  const rel: Relationship = {
    id: uid(),
    fromEntity: from,
    toEntity: to,
    fromCardinality: els.relFromCard.value as Cardinality,
    toCardinality: els.relToCard.value as Cardinality,
    label: els.relLabel.value.trim() || null,
    fromFields: [],
    toFields: [],
  };
  diagram.relationships.push(rel);
  els.relLabel.value = "";
  commitVisualEdit(`Added relationship ${from} → ${to}`);
  renderEditor();
}

function activateTab(tab: "edit" | "code") {
  document.querySelectorAll(".tab").forEach((el) => {
    const btn = el as HTMLButtonElement;
    const active = btn.dataset.tab === tab;
    btn.classList.toggle("active", active);
    btn.setAttribute("aria-selected", active ? "true" : "false");
  });
  const edit = $("tab-edit");
  const code = $("tab-code");
  if (tab === "edit") {
    edit.classList.add("active");
    edit.hidden = false;
    code.classList.remove("active");
    code.hidden = true;
  } else {
    code.classList.add("active");
    code.hidden = false;
    edit.classList.remove("active");
    edit.hidden = true;
    void syncCodeFromDiagram();
  }
}

function bind() {
  $("btn-sample").addEventListener("click", () => void onSample());
  $("btn-apply-code").addEventListener("click", () => void applyCode());
  $("btn-layout").addEventListener("click", () => void onLayout());
  $("btn-arrange-canvas").addEventListener("click", () => void onLayout());
  $("btn-validate").addEventListener("click", () => void onValidate());
  $("btn-export").addEventListener("click", () => void onExport());
  $("btn-copy").addEventListener("click", () => void onCopy());
  $("btn-import-file").addEventListener("click", onImportFile);
  $("btn-add-entity").addEventListener("click", addEntity);
  $("btn-add-attr").addEventListener("click", addAttribute);
  $("btn-delete-entity").addEventListener("click", deleteSelectedEntity);
  $("btn-add-rel").addEventListener("click", addRelationship);

  $("theme-select").addEventListener("change", (ev) => {
    const value = (ev.target as HTMLSelectElement).value as ThemeMode;
    applyTheme(value);
    setStatus(
      value === "system"
        ? "Theme: System"
        : value === "light"
          ? "Theme: Day"
          : "Theme: Dark",
    );
  });

  $("btn-zoom-in").addEventListener("click", () => {
    renderer.zoomBy(1.15);
    updateZoomLabel();
  });
  $("btn-zoom-out").addEventListener("click", () => {
    renderer.zoomBy(1 / 1.15);
    updateZoomLabel();
  });
  $("btn-zoom-fit").addEventListener("click", () => {
    renderer.fitToContent();
    updateZoomLabel();
  });
  $("btn-zoom-label").addEventListener("click", () => {
    renderer.setZoom(1);
    updateZoomLabel();
  });

  els.fieldEntityName.addEventListener("change", () => {
    if (!suppressNameInput) renameSelectedEntity(els.fieldEntityName.value);
  });
  els.fieldEntityName.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") {
      ev.preventDefault();
      renameSelectedEntity(els.fieldEntityName.value);
      (ev.target as HTMLInputElement).blur();
    }
  });

  document.querySelectorAll(".tab").forEach((el) => {
    el.addEventListener("click", () => {
      const tab = (el as HTMLElement).dataset.tab as "edit" | "code";
      activateTab(tab);
    });
  });

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

  document.body.addEventListener("dragover", (ev) => ev.preventDefault());
  document.body.addEventListener("drop", (ev) => {
    ev.preventDefault();
    const file = ev.dataTransfer?.files?.[0];
    if (file) void handleFile(file);
  });

  window.addEventListener("resize", () => updateZoomLabel());
}

async function boot() {
  initTheme();
  bind();
  updateZoomLabel();
  try {
    await onSample();
  } catch {
    setStatus("Ready — add an entity or paste Mermaid/DBML in Code");
  }
}

void boot();
