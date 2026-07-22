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
  /** Called when a drag commits a new position (model should update). */
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

function anchorOnSide(
  entity: Entity,
  side: Side,
  slot = 0.5,
): { x: number; y: number; side: Side } {
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
  const horizStart = start.side === "left" || start.side === "right";
  const horizEnd = end.side === "left" || end.side === "right";
  const pts: { x: number; y: number }[] = [
    { x: start.x, y: start.y },
    s,
  ];

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

function markerPath(card: Cardinality, x: number, y: number, side: Side): string {
  const outward = {
    left: { x: -1, y: 0 },
    right: { x: 1, y: 0 },
    top: { x: 0, y: -1 },
    bottom: { x: 0, y: 1 },
  }[side];
  const along = { x: -outward.y, y: outward.x };
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

/**
 * High-performance SVG renderer.
 *
 * Full rebuilds only happen when the diagram structure changes.
 * Dragging updates a single card transform + relationship paths (no DOM rebuild).
 * Panning/zooming only touch the root transform attribute.
 */
export class DiagramRenderer {
  private svg: SVGSVGElement;
  private root: SVGGElement;
  private edges: SVGGElement;
  private nodes: SVGGElement;
  private transform: ViewTransform = { x: 40, y: 40, k: 1 };
  private selected: string | null = null;
  private handlers: RenderHandlers;
  private diagram: Diagram | null = null;

  /** Live entity groups keyed by name — reused across paints. */
  private entityEls = new Map<string, SVGGElement>();
  /** Live relationship groups keyed by rel id — paths updated in place while dragging. */
  private edgeEls = new Map<string, SVGGElement>();

  private dragging: {
    name: string;
    ox: number;
    oy: number;
    startX: number;
    startY: number;
  } | null = null;
  private panning: { x: number; y: number; tx: number; ty: number } | null = null;
  private spaceDown = false;

  /** rAF coalesce for drag moves */
  private dragRaf = 0;
  private pendingDrag: { name: string; x: number; y: number } | null = null;

  /** Cached viewport size for zoom math (refreshed lazily). */
  private viewW = 0;
  private viewH = 0;

  constructor(svg: SVGSVGElement, handlers: RenderHandlers) {
    this.svg = svg;
    this.handlers = handlers;
    this.svg.innerHTML = "";

    this.root = document.createElementNS("http://www.w3.org/2000/svg", "g");
    this.edges = document.createElementNS("http://www.w3.org/2000/svg", "g");
    this.nodes = document.createElementNS("http://www.w3.org/2000/svg", "g");
    this.edges.setAttribute("class", "edges");
    this.nodes.setAttribute("class", "nodes");
    // Edges under nodes so cards stay clickable.
    this.root.append(this.edges, this.nodes);
    this.svg.appendChild(this.root);

    // Event delegation — one listener for all cards.
    this.nodes.addEventListener("pointerdown", this.onNodePointerDown);

    this.svg.addEventListener("pointerdown", this.onPointerDown);
    window.addEventListener("pointermove", this.onPointerMove, { passive: true });
    window.addEventListener("pointerup", this.onPointerUp);
    window.addEventListener("pointercancel", this.onPointerUp);
    this.svg.addEventListener("wheel", this.onWheel, { passive: false });
    window.addEventListener("keydown", this.onKeyDown);
    window.addEventListener("keyup", this.onKeyUp);

    const ro = new ResizeObserver(() => this.cacheViewSize());
    ro.observe(this.svg);
    this.cacheViewSize();
  }

  /** Selection highlight without rebuilding the scene. */
  setSelection(name: string | null) {
    if (this.selected === name) return;
    if (this.selected) {
      this.entityEls.get(this.selected)?.classList.remove("selected");
    }
    this.selected = name;
    if (name) {
      this.entityEls.get(name)?.classList.add("selected");
    }
  }

  getTransform(): ViewTransform {
    return { ...this.transform };
  }

  getZoom(): number {
    return this.transform.k;
  }

  zoomBy(factor: number, clientX?: number, clientY?: number) {
    this.cacheViewSize();
    const rect = this.svg.getBoundingClientRect();
    const mx = clientX != null ? clientX - rect.left : this.viewW / 2;
    const my = clientY != null ? clientY - rect.top : this.viewH / 2;
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
    this.cacheViewSize();
    const pad = 56;
    const w = maxX - minX || 1;
    const h = maxY - minY || 1;
    const kx = (this.viewW - pad * 2) / w;
    const ky = (this.viewH - pad * 2) / h;
    const k = Math.max(0.25, Math.min(1.6, Math.min(kx, ky)));
    this.transform.k = k;
    this.transform.x = (this.viewW - w * k) / 2 - minX * k;
    this.transform.y = (this.viewH - h * k) / 2 - minY * k;
    this.applyTransform();
    this.handlers.onZoomChange?.(k);
  }

  /** Full scene rebuild — call when structure/content changes, not on drag. */
  render(diagram: Diagram) {
    this.diagram = diagram;
    this.paint();
  }

  /**
   * Live position update during drag: moves one card + rewires edges only.
   * Does not rebuild entity DOM.
   */
  updateEntityPosition(name: string, x: number, y: number) {
    if (!this.diagram) return;
    const entity = this.diagram.entities.find((e) => e.name === name);
    if (!entity) return;
    entity.position = { x, y };

    const el = this.entityEls.get(name);
    if (el) {
      el.setAttribute("transform", `translate(${x},${y})`);
    }
    this.paintEdges();
  }

  private cacheViewSize() {
    const r = this.svg.getBoundingClientRect();
    this.viewW = r.width || this.viewW;
    this.viewH = r.height || this.viewH;
  }

  private applyTransform() {
    const { x, y, k } = this.transform;
    // Direct attribute write — cheap, GPU-friendly.
    this.root.setAttribute("transform", `translate(${x} ${y}) scale(${k})`);
  }

  private paint() {
    if (!this.diagram) return;
    this.applyTransform();
    this.paintEdges();
    this.paintEntities();
  }

  private paintEdges() {
    if (!this.diagram) return;

    const byName = new Map(this.diagram.entities.map((e) => [e.name, e]));

    // Precompute per-side attachment order so edges fan cleanly along card edges.
    // Key: `${entityName}|${side}` → relationship ids sorted by other endpoint Y.
    type SideKey = string;
    const sideBuckets = new Map<SideKey, { relId: string; otherY: number }[]>();

    const usable: { rel: Relationship; a: Entity; b: Entity; sides: { from: Side; to: Side } }[] =
      [];

    for (const rel of this.diagram.relationships) {
      const a = byName.get(rel.fromEntity);
      const b = byName.get(rel.toEntity);
      if (!a || !b) continue;
      const sides = pickSides(a, b);
      usable.push({ rel, a, b, sides });

      const ca = entityCenter(a);
      const cb = entityCenter(b);
      const fromKey = `${a.name}|${sides.from}`;
      const toKey = `${b.name}|${sides.to}`;
      if (!sideBuckets.has(fromKey)) sideBuckets.set(fromKey, []);
      if (!sideBuckets.has(toKey)) sideBuckets.set(toKey, []);
      sideBuckets.get(fromKey)!.push({ relId: rel.id, otherY: cb.y });
      sideBuckets.get(toKey)!.push({ relId: rel.id, otherY: ca.y });
    }

    // `${relId}@@${entityName}|${side}` → slot 0..1 along that card edge
    const sideSlot = new Map<string, number>();
    for (const [sideKey, list] of sideBuckets) {
      list.sort((p, q) => p.otherY - q.otherY || p.relId.localeCompare(q.relId));
      const n = list.length;
      list.forEach((item, i) => {
        // Spread ports between 0.22 and 0.78 so markers don't sit on corners.
        const slot = n === 1 ? 0.5 : 0.22 + (0.56 * i) / (n - 1);
        sideSlot.set(`${item.relId}@@${sideKey}`, slot);
      });
    }

    // Stagger mid-channel bends when many edges share a left→right corridor.
    const corridor = new Map<string, Relationship[]>();
    for (const { rel, a, b, sides } of usable) {
      if (
        (sides.from === "right" && sides.to === "left") ||
        (sides.from === "left" && sides.to === "right")
      ) {
        // Bucket by approximate column gap using rounded x.
        const lx = Math.round(Math.min(entityCenter(a).x, entityCenter(b).x) / 40) * 40;
        const key = `h:${lx}`;
        if (!corridor.has(key)) corridor.set(key, []);
        corridor.get(key)!.push(rel);
      }
    }
    const bendOf = new Map<string, number>();
    for (const [, rels] of corridor) {
      rels.sort((r1, r2) => {
        const a1 = byName.get(r1.fromEntity)!;
        const b1 = byName.get(r1.toEntity)!;
        const a2 = byName.get(r2.fromEntity)!;
        const b2 = byName.get(r2.toEntity)!;
        const m1 = (entityCenter(a1).y + entityCenter(b1).y) / 2;
        const m2 = (entityCenter(a2).y + entityCenter(b2).y) / 2;
        return m1 - m2 || r1.id.localeCompare(r2.id);
      });
      const n = rels.length;
      rels.forEach((rel, i) => {
        // Small alternating channel offsets so parallel ortholines don't stack.
        const bend = n <= 1 ? 0 : (i - (n - 1) / 2) * 14;
        bendOf.set(rel.id, bend);
      });
    }

    // Pair-level extra bend for multiple relationships between the same entities.
    const pairCount = new Map<string, number>();
    const pairIndex = new Map<string, number>();
    for (const { rel } of usable) {
      const key = pairKey(rel.fromEntity, rel.toEntity);
      pairCount.set(key, (pairCount.get(key) ?? 0) + 1);
    }

    const keep = new Set<string>();
    for (const { rel, a, b, sides } of usable) {
      const key = pairKey(rel.fromEntity, rel.toEntity);
      const total = pairCount.get(key) ?? 1;
      const idx = pairIndex.get(key) ?? 0;
      pairIndex.set(key, idx + 1);
      const pairBend = total === 1 ? 0 : (idx - (total - 1) / 2) * 22;
      const bend = (bendOf.get(rel.id) ?? 0) + pairBend;

      const fromSlot =
        sideSlot.get(`${rel.id}@@${a.name}|${sides.from}`) ?? 0.5;
      const toSlot =
        sideSlot.get(`${rel.id}@@${b.name}|${sides.to}`) ?? 0.5;
      keep.add(rel.id);

      let g = this.edgeEls.get(rel.id);
      if (!g) {
        g = this.buildRelationshipShell(rel);
        this.edges.appendChild(g);
        this.edgeEls.set(rel.id, g);
      }
      this.applyRelationshipGeometry(g, rel, a, b, bend, sides, fromSlot, toSlot);
    }

    for (const [id, el] of this.edgeEls) {
      if (!keep.has(id)) {
        el.remove();
        this.edgeEls.delete(id);
      }
    }
  }

  private buildRelationshipShell(rel: Relationship): SVGGElement {
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    g.setAttribute("class", "relationship");
    g.dataset.relId = rel.id;

    const hit = document.createElementNS("http://www.w3.org/2000/svg", "path");
    hit.setAttribute("class", "rel-hit");
    g.appendChild(hit);

    const line = document.createElementNS("http://www.w3.org/2000/svg", "path");
    line.setAttribute("class", "rel-line");
    g.appendChild(line);

    const m1 = document.createElementNS("http://www.w3.org/2000/svg", "path");
    m1.setAttribute("class", "rel-marker");
    g.appendChild(m1);

    const m2 = document.createElementNS("http://www.w3.org/2000/svg", "path");
    m2.setAttribute("class", "rel-marker");
    g.appendChild(m2);

    const bg = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    bg.setAttribute("class", "rel-label-bg");
    bg.setAttribute("height", "16");
    bg.setAttribute("rx", "8");
    bg.style.display = "none";
    g.appendChild(bg);

    const text = document.createElementNS("http://www.w3.org/2000/svg", "text");
    text.setAttribute("class", "rel-label");
    text.style.display = "none";
    g.appendChild(text);

    return g;
  }

  private applyRelationshipGeometry(
    g: SVGGElement,
    rel: Relationship,
    a: Entity,
    b: Entity,
    bend: number,
    sidesArg?: { from: Side; to: Side },
    fromSlot = 0.5,
    toSlot = 0.5,
  ) {
    const sides = sidesArg ?? pickSides(a, b);
    const start = anchorOnSide(a, sides.from, fromSlot);
    const end = anchorOnSide(b, sides.to, toSlot);
    const path = orthoPath(start, end, bend);

    const kids = g.children;
    // hit, line, m1, m2, bg, text
    (kids[0] as SVGPathElement).setAttribute("d", path);
    (kids[1] as SVGPathElement).setAttribute("d", path);
    (kids[2] as SVGPathElement).setAttribute(
      "d",
      markerPath(rel.fromCardinality, start.x, start.y, start.side),
    );
    (kids[3] as SVGPathElement).setAttribute(
      "d",
      markerPath(rel.toCardinality, end.x, end.y, end.side),
    );

    const bg = kids[4] as SVGRectElement;
    const text = kids[5] as SVGTextElement;
    if (rel.label) {
      const lx = (start.x + end.x) / 2 + bend * 0.25;
      const ly = (start.y + end.y) / 2 - 8;
      const label = rel.label.length > 28 ? `${rel.label.slice(0, 26)}…` : rel.label;
      const tw = label.length * 6.2 + 12;
      bg.style.display = "";
      text.style.display = "";
      bg.setAttribute("x", String(lx - tw / 2));
      bg.setAttribute("y", String(ly - 11));
      bg.setAttribute("width", String(tw));
      text.setAttribute("x", String(lx));
      text.setAttribute("y", String(ly));
      text.textContent = label;
    } else {
      bg.style.display = "none";
      text.style.display = "none";
      text.textContent = "";
    }
  }

  private paintEntities() {
    if (!this.diagram) return;
    const keep = new Set(this.diagram.entities.map((e) => e.name));

    // Remove stale cards.
    for (const [name, el] of this.entityEls) {
      if (!keep.has(name)) {
        el.remove();
        this.entityEls.delete(name);
      }
    }

    // Rebuild or create cards. Structure changes need full card rebuild.
    for (const entity of this.diagram.entities) {
      const existing = this.entityEls.get(entity.name);
      if (existing) {
        // Replace contents in place to keep node identity stable for selection.
        const fresh = this.buildEntity(entity);
        existing.replaceWith(fresh);
        this.entityEls.set(entity.name, fresh);
      } else {
        const g = this.buildEntity(entity);
        this.nodes.appendChild(g);
        this.entityEls.set(entity.name, g);
      }
    }
  }

  private buildEntity(entity: Entity): SVGGElement {
    const pos = entity.position ?? { x: 0, y: 0 };
    const h = cardHeight(entity);
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    g.setAttribute(
      "class",
      `entity-card${this.selected === entity.name ? " selected" : ""}`,
    );
    g.setAttribute("transform", `translate(${pos.x},${pos.y})`);
    g.dataset.entity = entity.name;
    g.style.cursor = "grab";

    const shadow = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    shadow.setAttribute("x", "2");
    shadow.setAttribute("y", "3");
    shadow.setAttribute("width", String(CARD_W));
    shadow.setAttribute("height", String(h));
    shadow.setAttribute("rx", "10");
    shadow.setAttribute("class", "card-shadow");
    // Shadows must not capture pointer events.
    shadow.setAttribute("pointer-events", "none");
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
    header.setAttribute("pointer-events", "none");
    g.appendChild(header);
    const headerFix = document.createElementNS("http://www.w3.org/2000/svg", "rect");
    headerFix.setAttribute("y", String(HEADER_H - 10));
    headerFix.setAttribute("width", String(CARD_W));
    headerFix.setAttribute("height", "10");
    headerFix.setAttribute("class", "card-header");
    headerFix.setAttribute("pointer-events", "none");
    g.appendChild(headerFix);

    const title = document.createElementNS("http://www.w3.org/2000/svg", "text");
    title.textContent = entity.name;
    title.setAttribute("x", String(PAD_X));
    title.setAttribute("y", "24");
    title.setAttribute("class", "card-title");
    title.setAttribute("pointer-events", "none");
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

    for (let i = 0; i < attrs.length; i++) {
      const attr = attrs[i];
      const y = HEADER_H + 16 + i * ATTR_H;
      const name = document.createElementNS("http://www.w3.org/2000/svg", "text");
      name.setAttribute("x", String(PAD_X));
      name.setAttribute("y", String(y));
      name.setAttribute(
        "class",
        `attr-name${attr.isPk ? " pk" : ""}${attr.isFk ? " fk" : ""}`,
      );
      name.textContent = attr.name;
      name.setAttribute("pointer-events", "none");
      g.appendChild(name);

      const meta = document.createElementNS("http://www.w3.org/2000/svg", "text");
      meta.setAttribute("x", String(CARD_W - PAD_X));
      meta.setAttribute("y", String(y));
      meta.setAttribute("text-anchor", "end");
      meta.setAttribute("class", "attr-meta");
      meta.setAttribute("pointer-events", "none");
      const marker = cardMarker(attr as Entity["attributes"][number]);
      meta.textContent = marker || attr.dataType || "";
      g.appendChild(meta);
    }

    return g;
  }

  private clearBrowserSelection() {
    const sel = window.getSelection?.();
    if (sel && sel.rangeCount > 0) sel.removeAllRanges();
  }

  private setInteracting(on: boolean) {
    document.body.classList.toggle("is-canvas-interacting", on);
    if (on) this.clearBrowserSelection();
  }

  private onNodePointerDown = (ev: PointerEvent) => {
    if (ev.button !== 0 || this.spaceDown) return;
    const target = ev.target as Element | null;
    const g = target?.closest?.("[data-entity]") as SVGGElement | null;
    if (!g || !this.diagram) return;

    const name = g.dataset.entity;
    if (!name) return;

    ev.stopPropagation();
    ev.preventDefault();
    this.clearBrowserSelection();

    const entity = this.diagram.entities.find((e) => e.name === name);
    if (!entity) return;

    this.setSelection(name);
    this.handlers.onSelectEntity(name);

    const p = entity.position ?? { x: 0, y: 0 };
    this.dragging = {
      name,
      ox: p.x,
      oy: p.y,
      startX: ev.clientX,
      startY: ev.clientY,
    };
    g.style.cursor = "grabbing";
    this.svg.classList.add("dragging");
    this.setInteracting(true);
    try {
      this.svg.setPointerCapture(ev.pointerId);
    } catch {
      /* ignore */
    }
  };

  private onKeyDown = (ev: KeyboardEvent) => {
    const t = ev.target as HTMLElement | null;
    const typing =
      t &&
      (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable);
    if (ev.code === "Space" && !typing) {
      this.spaceDown = true;
      this.svg.classList.add("panning");
      ev.preventDefault();
    }
    if (typing) return;
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
      if (!this.panning) this.svg.classList.remove("panning");
    }
  };

  private onPointerDown = (ev: PointerEvent) => {
    if (ev.button === 1 || (ev.button === 0 && (this.spaceDown || ev.altKey))) {
      ev.preventDefault();
      this.clearBrowserSelection();
      this.panning = {
        x: ev.clientX,
        y: ev.clientY,
        tx: this.transform.x,
        ty: this.transform.y,
      };
      this.svg.classList.add("panning");
      this.setInteracting(true);
      try {
        this.svg.setPointerCapture(ev.pointerId);
      } catch {
        /* ignore */
      }
      return;
    }
    if (ev.button !== 0) return;
    // Background drag = pan. Stop the browser from selecting SVG text.
    ev.preventDefault();
    this.clearBrowserSelection();
    this.setSelection(null);
    this.handlers.onSelectEntity(null);
    this.panning = {
      x: ev.clientX,
      y: ev.clientY,
      tx: this.transform.x,
      ty: this.transform.y,
    };
    this.svg.classList.add("panning");
    this.setInteracting(true);
    try {
      this.svg.setPointerCapture(ev.pointerId);
    } catch {
      /* ignore */
    }
  };

  private onPointerMove = (ev: PointerEvent) => {
    if (this.dragging || this.panning) {
      // Keep selection cleared if the OS tries to extend it mid-gesture.
      this.clearBrowserSelection();
    }
    if (this.dragging) {
      const dx = (ev.clientX - this.dragging.startX) / this.transform.k;
      const dy = (ev.clientY - this.dragging.startY) / this.transform.k;
      const x = this.dragging.ox + dx;
      const y = this.dragging.oy + dy;
      // Coalesce to one visual update per frame.
      this.pendingDrag = { name: this.dragging.name, x, y };
      if (!this.dragRaf) {
        this.dragRaf = requestAnimationFrame(this.flushDrag);
      }
      return;
    }
    if (this.panning) {
      this.transform.x = this.panning.tx + (ev.clientX - this.panning.x);
      this.transform.y = this.panning.ty + (ev.clientY - this.panning.y);
      this.applyTransform();
    }
  };

  private flushDrag = () => {
    this.dragRaf = 0;
    const pending = this.pendingDrag;
    if (!pending) return;
    this.pendingDrag = null;
    // Visual + in-renderer model update (fast path).
    this.updateEntityPosition(pending.name, pending.x, pending.y);
    // Sync app model (no re-render).
    this.handlers.onMoveEntity(pending.name, pending.x, pending.y);
  };

  private onPointerUp = () => {
    if (this.dragRaf) {
      cancelAnimationFrame(this.dragRaf);
      this.dragRaf = 0;
    }
    // Flush last drag sample.
    if (this.pendingDrag) {
      const p = this.pendingDrag;
      this.pendingDrag = null;
      this.updateEntityPosition(p.name, p.x, p.y);
      this.handlers.onMoveEntity(p.name, p.x, p.y);
    }
    if (this.dragging) {
      const el = this.entityEls.get(this.dragging.name);
      if (el) el.style.cursor = "grab";
    }
    this.dragging = null;
    this.panning = null;
    this.svg.classList.remove("dragging");
    if (!this.spaceDown) this.svg.classList.remove("panning");
    this.setInteracting(false);
    this.clearBrowserSelection();
  };

  private onWheel = (ev: WheelEvent) => {
    ev.preventDefault();
    this.clearBrowserSelection();
    if (ev.shiftKey) {
      this.transform.x -= ev.deltaY;
      this.transform.y -= ev.deltaX;
      this.applyTransform();
      return;
    }
    const factor = ev.deltaY < 0 ? 1.08 : 1 / 1.08;
    this.zoomBy(factor, ev.clientX, ev.clientY);
  };
}
