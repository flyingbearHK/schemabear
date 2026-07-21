import type { Cardinality, Diagram, Entity, Relationship } from "../types";

export const CARD_W = 240;
const HEADER_H = 36;
const ATTR_H = 22;
const PAD_X = 12;

export interface ViewTransform {
  x: number;
  y: number;
  k: number;
}

export interface RenderHandlers {
  onSelectEntity: (name: string | null) => void;
  onMoveEntity: (name: string, x: number, y: number) => void;
  onZoomChange?: (k: number) => void;
}

export function cardHeight(entity: Entity): number {
  const n = Math.max(entity.attributes.length, 1);
  return HEADER_H + n * ATTR_H + 8;
}

function cardMarker(attr: Entity["attributes"][number]): string {
  const marks: string[] = [];
  if (attr.isPk) marks.push("PK");
  if (attr.isFk) marks.push("FK");
  if (attr.isUnique && !attr.isPk) marks.push("UK");
  return marks.join(" ");
}

type Side = "left" | "right" | "top" | "bottom";

function entityCenter(entity: Entity): { x: number; y: number } {
  const pos = entity.position ?? { x: 0, y: 0 };
  return { x: pos.x + CARD_W / 2, y: pos.y + cardHeight(entity) / 2 };
}

function anchorOnSide(entity: Entity, side: Side, slot = 0.5): { x: number; y: number; side: Side } {
  const pos = entity.position ?? { x: 0, y: 0 };
  const h = cardHeight(entity);
  const t = Math.min(0.85, Math.max(0.15, slot));
  switch (side) {
    case "left":
      return { x: pos.x, y: pos.y + h * t, side };
    case "right":
      return { x: pos.x + CARD_W, y: pos.y + h * t, side };
    case "top":
      return { x: pos.x + CARD_W * t, y: pos.y, side };
    case "bottom":
      return { x: pos.x + CARD_W * t, y: pos.y + h, side };
  }
}

function pickSides(a: Entity, b: Entity): { from: Side; to: Side } {
  const ca = entityCenter(a);
  const cb = entityCenter(b);
  const dx = cb.x - ca.x;
  const dy = cb.y - ca.y;
  if (Math.abs(dx) >= Math.abs(dy)) {
    return dx >= 0 ? { from: "right", to: "left" } : { from: "left", to: "right" };
  }
  return dy >= 0 ? { from: "bottom", to: "top" } : { from: "top", to: "bottom" };
}

/** Orthogonal connector with rounded elbows. */
function orthoPath(
  start: { x: number; y: number; side: Side },
  end: { x: number; y: number; side: Side },
  bend = 0,
): string {
  const pad = 18;
  const exit = (p: { x: number; y: number; side: Side }) => {
    switch (p.side) {
      case "left":
        return { x: p.x - pad, y: p.y };
      case "right":
        return { x: p.x + pad, y: p.y };
      case "top":
        return { x: p.x, y: p.y - pad };
      case "bottom":
        return { x: p.x, y: p.y + pad };
    }
  };
  const s = exit(start);
  const e = exit(end);
  const midX = (s.x + e.x) / 2 + bend;
  const r = 10;

  // Horizontal-first routing when endpoints are on left/right.
  const horizStart = start.side === "left" || start.side === "right";
  const horizEnd = end.side === "left" || end.side === "right";

  const pts: { x: number; y: number }[] = [{ x: start.x, y: start.y }, s];

  if (horizStart && horizEnd) {
    pts.push({ x: midX, y: s.y }, { x: midX, y: e.y });
  } else if (!horizStart && !horizEnd) {
    const midY = (s.y + e.y) / 2 + bend;
    pts.push({ x: s.x, y: midY }, { x: e.x, y: midY });
  } else if (horizStart) {
    pts.push({ x: e.x, y: s.y });
  } else {
    pts.push({ x: s.x, y: e.y });
  }

  pts.push(e, { x: end.x, y: end.y });

  return roundedPolyline(pts, r);
}

function roundedPolyline(pts: { x: number; y: number }[], radius: number): string {
  if (pts.length < 2) return "";
  let d = `M ${pts[0].x} ${pts[0].y}`;
  for (let i = 1; i < pts.length; i++) {
    const prev = pts[i - 1];
    const curr = pts[i];
    const next = pts[i + 1];
    if (!next) {
      d += ` L ${curr.x} ${curr.y}`;
      break;
    }
    const v1x = curr.x - prev.x;
    const v1y = curr.y - prev.y;
    const v2x = next.x - curr.x;
    const v2y = next.y - curr.y;
    const len1 = Math.hypot(v1x, v1y) || 1;
    const len2 = Math.hypot(v2x, v2y) || 1;
    const r = Math.min(radius, len1 / 2, len2 / 2);
    const p1x = curr.x - (v1x / len1) * r;
    const p1y = curr.y - (v1y / len1) * r;
    const p2x = curr.x + (v2x / len2) * r;
    const p2y = curr.y + (v2y / len2) * r;
    d += ` L ${p1x} ${p1y} Q ${curr.x} ${curr.y} ${p2x} ${p2y}`;
  }
  return d;
}

/** Crow's-foot / bar marker oriented by side normal (outward from entity). */
function markerPath(card: Cardinality, x: number, y: number, side: Side): string {
  const outward = {
    left: { x: -1, y: 0 },
    right: { x: 1, y: 0 },
    top: { x: 0, y: -1 },
    bottom: { x: 0, y: 1 },
  }[side];
  const along = { x: -outward.y, y: outward.x }; // perpendicular
  const ox = outward.x;
  const oy = outward.y;
  const ax = along.x;
  const ay = along.y;

  const bar = (dist: number, half = 7) => {
    const cx = x + ox * dist;
    const cy = y + oy * dist;
    return `M ${cx + ax * half} ${cy + ay * half} L ${cx - ax * half} ${cy - ay * half}`;
  };
  const circle = (dist: number, r = 3.8) => {
    const cx = x + ox * dist;
    const cy = y + oy * dist;
    return `M ${cx + r} ${cy} A ${r} ${r} 0 1 1 ${cx - r} ${cy} A ${r} ${r} 0 1 1 ${cx + r} ${cy}`;
  };
  const crow = (dist: number) => {
    const baseX = x + ox * dist;
    const baseY = y + oy * dist;
    const tipX = baseX + ox * 11;
    const tipY = baseY + oy * 11;
    const w = 7;
    return [
      `M ${baseX} ${baseY} L ${tipX + ax * w} ${tipY + ay * w}`,
      `M ${baseX} ${baseY} L ${tipX} ${tipY}`,
      `M ${baseX} ${baseY} L ${tipX - ax * w} ${tipY - ay * w}`,
    ].join(" ");
  };

  switch (card) {
    case "one":
      return bar(5);
    case "zero_or_one":
      return `${circle(4)}${bar(12)}`;
    case "one_or_many":
      return `${bar(4)}${crow(8)}`;
    case "zero_or_many":
      return `${circle(4)}${crow(12)}`;
  }
}

function pairKey(a: string, b: string): string {
  return a < b ? `${a}::${b}` : `${b}::${a}`;
}

export class DiagramRenderer {
  private svg: SVGSVGElement;
  private root: SVGGElement;
  private edges: SVGGElement;
  private nodes: SVGGElement;
  private transform: ViewTransform = { x: 40, y: 40, k: 1 };
  private selected: string | null = null;
  private handlers: RenderHandlers;
  private diagram: Diagram | null = null;
  private dragging: {
    name: string;
    ox: number;
    oy: number;
    startX: number;
    startY: number;
  } | null = null;
  private panning: { x: number; y: number; tx: number; ty: number } | null = null;
  private spaceDown = false;

  constructor(svg: SVGSVGElement, handlers: RenderHandlers) {
    this.svg = svg;
    this.handlers = handlers;
    this.svg.innerHTML = "";

    // defs for subtle edge shadow
    const defs = document.createElementNS("http://www.w3.org/2000/svg", "defs");
    defs.innerHTML = `
      <filter id="edge-soft" x="-20%" y="-20%" width="140%" height="140%">
        <feDropShadow dx="0" dy="0.5" stdDeviation="0.6" flood-opacity="0.18"/>
      </filter>
    `;
    this.svg.appendChild(defs);

    this.root = document.createElementNS("http://www.w3.org/2000/svg", "g");
    this.edges = document.createElementNS("http://www.w3.org/2000/svg", "g");
    this.nodes = document.createElementNS("http://www.w3.org/2000/svg", "g");
    this.edges.setAttribute("class", "edges");
    this.nodes.setAttribute("class", "nodes");
    this.root.append(this.edges, this.nodes);
    this.svg.appendChild(this.root);

    this.svg.addEventListener("pointerdown", this.onPointerDown);
    window.addEventListener("pointermove", this.onPointerMove);
    window.addEventListener("pointerup", this.onPointerUp);
    this.svg.addEventListener("wheel", this.onWheel, { passive: false });
    window.addEventListener("keydown", this.onKeyDown);
    window.addEventListener("keyup", this.onKeyUp);
  }

  setSelection(name: string | null) {
    this.selected = name;
    this.paint();
  }

  getTransform(): ViewTransform {
    return { ...this.transform };
  }

  getZoom(): number {
    return this.transform.k;
  }

  /** Zoom by factor around viewport center or optional client point. */
  zoomBy(factor: number, clientX?: number, clientY?: number) {
    const rect = this.svg.getBoundingClientRect();
    const mx = clientX != null ? clientX - rect.left : rect.width / 2;
    const my = clientY != null ? clientY - rect.top : rect.height / 2;
    const oldK = this.transform.k;
    const next = Math.min(3, Math.max(0.25, oldK * factor));
    if (next === oldK) return;
    const wx = (mx - this.transform.x) / oldK;
    const wy = (my - this.transform.y) / oldK;
    this.transform.k = next;
    this.transform.x = mx - wx * next;
    this.transform.y = my - wy * next;
    this.applyTransform();
    this.handlers.onZoomChange?.(next);
  }

  setZoom(k: number) {
    this.zoomBy(k / this.transform.k);
  }

  fitToContent() {
    if (!this.diagram || this.diagram.entities.length === 0) return;
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity,
      maxY = -Infinity;
    for (const e of this.diagram.entities) {
      const p = e.position ?? { x: 0, y: 0 };
      const h = cardHeight(e);
      minX = Math.min(minX, p.x);
      minY = Math.min(minY, p.y);
      maxX = Math.max(maxX, p.x + CARD_W);
      maxY = Math.max(maxY, p.y + h);
    }
    const vb = this.svg.getBoundingClientRect();
    const pad = 56;
    const w = maxX - minX || 1;
    const h = maxY - minY || 1;
    const kx = (vb.width - pad * 2) / w;
    const ky = (vb.height - pad * 2) / h;
    const k = Math.max(0.25, Math.min(1.6, Math.min(kx, ky)));
    this.transform.k = k;
    this.transform.x = (vb.width - w * k) / 2 - minX * k;
    this.transform.y = (vb.height - h * k) / 2 - minY * k;
    this.applyTransform();
    this.handlers.onZoomChange?.(k);
  }

  render(diagram: Diagram) {
    this.diagram = diagram;
    this.paint();
  }

  private applyTransform() {
    const { x, y, k } = this.transform;
    this.root.setAttribute("transform", `translate(${x},${y}) scale(${k})`);
  }

  private paint() {
    if (!this.diagram) return;
    this.applyTransform();
    this.edges.innerHTML = "";
    this.nodes.innerHTML = "";

    const byName = new Map(this.diagram.entities.map((e) => [e.name, e]));

    // Offset parallel edges between the same pair.
    const pairCount = new Map<string, number>();
    const pairIndex = new Map<string, number>();
    for (const rel of this.diagram.relationships) {
      const key = pairKey(rel.fromEntity, rel.toEntity);
      pairCount.set(key, (pairCount.get(key) ?? 0) + 1);
    }

    for (const rel of this.diagram.relationships) {
      const a = byName.get(rel.fromEntity);
      const b = byName.get(rel.toEntity);
      if (!a || !b) continue;
      const key = pairKey(rel.fromEntity, rel.toEntity);
      const total = pairCount.get(key) ?? 1;
      const idx = pairIndex.get(key) ?? 0;
      pairIndex.set(key, idx + 1);
      const bend = total === 1 ? 0 : (idx - (total - 1) / 2) * 24;
      this.drawRelationship(rel, a, b, bend);
    }

    for (const entity of this.diagram.entities) {
      this.drawEntity(entity);
    }
  }

  private drawRelationship(rel: Relationship, a: Entity, b: Entity, bend: number) {
    const sides = pickSides(a, b);
    // Stagger anchors slightly when bend != 0
    const slot = 0.5 + Math.max(-0.25, Math.min(0.25, bend / 120));
    const start = anchorOnSide(a, sides.from, slot);
    const end = anchorOnSide(b, sides.to, 1 - slot);
    const path = orthoPath(start, end, bend);

    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    g.setAttribute("class", "relationship");

    const hit = document.createElementNS("http://www.w3.org/2000/svg", "path");
    hit.setAttribute("d", path);
    hit.setAttribute("class", "rel-hit");
    g.appendChild(hit);

    const line = document.createElementNS("http://www.w3.org/2000/svg", "path");
    line.setAttribute("d", path);
    line.setAttribute("class", "rel-line");
    line.setAttribute("filter", "url(#edge-soft)");
    g.appendChild(line);

    const m1 = document.createElementNS("http://www.w3.org/2000/svg", "path");
    m1.setAttribute("d", markerPath(rel.fromCardinality, start.x, start.y, start.side));
    m1.setAttribute("class", "rel-marker");
    g.appendChild(m1);

    const m2 = document.createElementNS("http://www.w3.org/2000/svg", "path");
    m2.setAttribute("d", markerPath(rel.toCardinality, end.x, end.y, end.side));
    m2.setAttribute("class", "rel-marker");
    g.appendChild(m2);

    if (rel.label) {
      const lx = (start.x + end.x) / 2 + bend * 0.25;
      const ly = (start.y + end.y) / 2 - 8;
      const label = rel.label.length > 28 ? `${rel.label.slice(0, 26)}…` : rel.label;
      const bg = document.createElementNS("http://www.w3.org/2000/svg", "rect");
      const text = document.createElementNS("http://www.w3.org/2000/svg", "text");
      text.textContent = label;
      text.setAttribute("x", String(lx));
      text.setAttribute("y", String(ly));
      text.setAttribute("class", "rel-label");
      // approximate pill
      const tw = label.length * 6.2 + 12;
      bg.setAttribute("x", String(lx - tw / 2));
      bg.setAttribute("y", String(ly - 11));
      bg.setAttribute("width", String(tw));
      bg.setAttribute("height", "16");
      bg.setAttribute("rx", "8");
      bg.setAttribute("class", "rel-label-bg");
      g.append(bg, text);
    }

    this.edges.appendChild(g);
  }

  private drawEntity(entity: Entity) {
    const pos = entity.position ?? { x: 0, y: 0 };
    const h = cardHeight(entity);
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    g.setAttribute(
      "class",
      `entity-card${this.selected === entity.name ? " selected" : ""}`,
    );
    g.setAttribute("transform", `translate(${pos.x},${pos.y})`);
    g.style.cursor = "grab";

    const shadow = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    shadow.setAttribute("x", "2");
    shadow.setAttribute("y", "3");
    shadow.setAttribute("width", String(CARD_W));
    shadow.setAttribute("height", String(h));
    shadow.setAttribute("rx", "10");
    shadow.setAttribute("class", "card-shadow");
    g.appendChild(shadow);

    const rect = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    rect.setAttribute("width", String(CARD_W));
    rect.setAttribute("height", String(h));
    rect.setAttribute("rx", "10");
    rect.setAttribute("class", "card-body");
    g.appendChild(rect);

    const header = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    header.setAttribute("width", String(CARD_W));
    header.setAttribute("height", String(HEADER_H));
    header.setAttribute("rx", "10");
    header.setAttribute("class", "card-header");
    g.appendChild(header);
    const headerFix = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    headerFix.setAttribute("y", String(HEADER_H - 10));
    headerFix.setAttribute("width", String(CARD_W));
    headerFix.setAttribute("height", "10");
    headerFix.setAttribute("class", "card-header");
    g.appendChild(headerFix);

    const title = document.createElementNS("http://www.w3.org/2000/svg", "text");
    title.textContent = entity.name;
    title.setAttribute("x", String(PAD_X));
    title.setAttribute("y", "24");
    title.setAttribute("class", "card-title");
    g.appendChild(title);

    const attrs = entity.attributes.length
      ? entity.attributes
      : [
          {
            name: "(no attributes)",
            dataType: "",
            isPk: false,
            isFk: false,
            isUnique: false,
            isNullable: true,
          },
        ];

    attrs.forEach((attr, i) => {
      const y = HEADER_H + 16 + i * ATTR_H;
      const name = document.createElementNS("http://www.w3.org/2000/svg", "text");
      name.setAttribute("x", String(PAD_X));
      name.setAttribute("y", String(y));
      name.setAttribute("class", `attr-name${attr.isPk ? " pk" : ""}${attr.isFk ? " fk" : ""}`);
      name.textContent = attr.name;
      g.appendChild(name);

      const meta = document.createElementNS("http://www.w3.org/2000/svg", "text");
      meta.setAttribute("x", String(CARD_W - PAD_X));
      meta.setAttribute("y", String(y));
      meta.setAttribute("text-anchor", "end");
      meta.setAttribute("class", "attr-meta");
      const marker = cardMarker(attr as Entity["attributes"][number]);
      meta.textContent = marker || attr.dataType || "";
      g.appendChild(meta);
    });

    g.addEventListener("pointerdown", (ev) => {
      if (ev.button !== 0) return;
      ev.stopPropagation();
      this.handlers.onSelectEntity(entity.name);
      this.selected = entity.name;
      const p = entity.position ?? { x: 0, y: 0 };
      this.dragging = {
        name: entity.name,
        ox: p.x,
        oy: p.y,
        startX: ev.clientX,
        startY: ev.clientY,
      };
      this.svg.setPointerCapture?.(ev.pointerId);
      this.paint();
    });

    this.nodes.appendChild(g);
  }

  private onKeyDown = (ev: KeyboardEvent) => {
    if (ev.code === "Space") {
      // Don't steal space from inputs.
      const t = ev.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) {
        return;
      }
      this.spaceDown = true;
      this.svg.classList.add("panning");
      ev.preventDefault();
    }
    if ((ev.target as HTMLElement)?.tagName === "INPUT" || (ev.target as HTMLElement)?.tagName === "TEXTAREA") {
      return;
    }
    if (ev.key === "+" || ev.key === "=") {
      this.zoomBy(1.15);
      ev.preventDefault();
    } else if (ev.key === "-" || ev.key === "_") {
      this.zoomBy(1 / 1.15);
      ev.preventDefault();
    } else if (ev.key === "0") {
      this.fitToContent();
      ev.preventDefault();
    }
  };

  private onKeyUp = (ev: KeyboardEvent) => {
    if (ev.code === "Space") {
      this.spaceDown = false;
      this.svg.classList.remove("panning");
    }
  };

  private onPointerDown = (ev: PointerEvent) => {
    if (ev.button === 1 || (ev.button === 0 && (this.spaceDown || ev.altKey))) {
      this.panning = {
        x: ev.clientX,
        y: ev.clientY,
        tx: this.transform.x,
        ty: this.transform.y,
      };
      this.svg.classList.add("panning");
      ev.preventDefault();
      return;
    }
    if (ev.button !== 0) return;
    this.handlers.onSelectEntity(null);
    this.selected = null;
    this.panning = {
      x: ev.clientX,
      y: ev.clientY,
      tx: this.transform.x,
      ty: this.transform.y,
    };
    this.paint();
  };

  private onPointerMove = (ev: PointerEvent) => {
    if (this.dragging) {
      const dx = (ev.clientX - this.dragging.startX) / this.transform.k;
      const dy = (ev.clientY - this.dragging.startY) / this.transform.k;
      this.handlers.onMoveEntity(
        this.dragging.name,
        this.dragging.ox + dx,
        this.dragging.oy + dy,
      );
      return;
    }
    if (this.panning) {
      this.transform.x = this.panning.tx + (ev.clientX - this.panning.x);
      this.transform.y = this.panning.ty + (ev.clientY - this.panning.y);
      this.applyTransform();
    }
  };

  private onPointerUp = () => {
    this.dragging = null;
    this.panning = null;
    if (!this.spaceDown) this.svg.classList.remove("panning");
  };

  private onWheel = (ev: WheelEvent) => {
    ev.preventDefault();
    // Default: zoom toward cursor. Shift = pan horizontally/vertically.
    if (ev.shiftKey) {
      this.transform.x -= ev.deltaY;
      this.transform.y -= ev.deltaX;
      this.applyTransform();
      return;
    }
    // Trackpads often send ctrlKey with pinch-zoom already.
    const factor = ev.deltaY < 0 ? 1.08 : 1 / 1.08;
    this.zoomBy(factor, ev.clientX, ev.clientY);
  };
}
