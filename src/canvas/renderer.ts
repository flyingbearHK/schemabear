import type { Cardinality, Diagram, Entity, Relationship } from "../types";

const CARD_W = 220;
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
}

function cardHeight(entity: Entity): number {
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

function cardinalityPath(card: Cardinality, at: "start" | "end", x: number, y: number, dx: number, dy: number): string {
  // Simple crow's-foot / bar markers near the endpoint.
  const len = 10;
  const nx = dx === 0 ? 0 : dx / Math.abs(dx || 1);
  const ny = dy === 0 ? 0 : dy / Math.abs(dy || 1);
  // Orthogonal preference: use dominant axis.
  const horizontal = Math.abs(dx) >= Math.abs(dy);
  const tx = horizontal ? -Math.sign(dx || 1) : 0;
  const ty = horizontal ? 0 : -Math.sign(dy || 1);
  const px = -ty;
  const py = tx;

  const bx = x + tx * 4;
  const by = y + ty * 4;

  if (card === "one") {
    const x1 = bx + px * len * 0.5;
    const y1 = by + py * len * 0.5;
    const x2 = bx - px * len * 0.5;
    const y2 = by - py * len * 0.5;
    return `M ${x1} ${y1} L ${x2} ${y2}`;
  }

  if (card === "zero_or_one") {
    const cx = bx + tx * 6;
    const cy = by + ty * 6;
    const x1 = bx + px * len * 0.5;
    const y1 = by + py * len * 0.5;
    const x2 = bx - px * len * 0.5;
    const y2 = by - py * len * 0.5;
    return `M ${x1} ${y1} L ${x2} ${y2} M ${cx + 3.5} ${cy} A 3.5 3.5 0 1 1 ${cx - 3.5} ${cy} A 3.5 3.5 0 1 1 ${cx + 3.5} ${cy}`;
  }

  // many variants: crow's foot
  const baseX = bx;
  const baseY = by;
  const tipX = baseX + tx * len;
  const tipY = baseY + ty * len;
  const wing = 6;
  const a1x = tipX + px * wing;
  const a1y = tipY + py * wing;
  const a2x = tipX - px * wing;
  const a2y = tipY - py * wing;
  let d = `M ${baseX} ${baseY} L ${a1x} ${a1y} M ${baseX} ${baseY} L ${tipX} ${tipY} M ${baseX} ${baseY} L ${a2x} ${a2y}`;
  if (card === "zero_or_many") {
    const cx = baseX - tx * 8;
    const cy = baseY - ty * 8;
    d += ` M ${cx + 3.5} ${cy} A 3.5 3.5 0 1 1 ${cx - 3.5} ${cy} A 3.5 3.5 0 1 1 ${cx + 3.5} ${cy}`;
  } else if (card === "one_or_many") {
    const x1 = baseX - tx * 6 + px * len * 0.45;
    const y1 = baseY - ty * 6 + py * len * 0.45;
    const x2 = baseX - tx * 6 - px * len * 0.45;
    const y2 = baseY - ty * 6 - py * len * 0.45;
    d += ` M ${x1} ${y1} L ${x2} ${y2}`;
  }
  void at;
  void nx;
  void ny;
  return d;
}

function anchorPoint(entity: Entity, toward: { x: number; y: number }): { x: number; y: number } {
  const pos = entity.position ?? { x: 0, y: 0 };
  const h = cardHeight(entity);
  const cx = pos.x + CARD_W / 2;
  const cy = pos.y + h / 2;
  const dx = toward.x - cx;
  const dy = toward.y - cy;
  if (Math.abs(dx) > Math.abs(dy)) {
    return {
      x: dx > 0 ? pos.x + CARD_W : pos.x,
      y: cy,
    };
  }
  return {
    x: cx,
    y: dy > 0 ? pos.y + h : pos.y,
  };
}

function entityCenter(entity: Entity): { x: number; y: number } {
  const pos = entity.position ?? { x: 0, y: 0 };
  return { x: pos.x + CARD_W / 2, y: pos.y + cardHeight(entity) / 2 };
}

export class DiagramRenderer {
  private svg: SVGSVGElement;
  private root: SVGGElement;
  private edges: SVGGElement;
  private nodes: SVGGElement;
  private transform: ViewTransform = { x: 0, y: 0, k: 1 };
  private selected: string | null = null;
  private handlers: RenderHandlers;
  private diagram: Diagram | null = null;
  private dragging: { name: string; ox: number; oy: number; startX: number; startY: number } | null = null;
  private panning: { x: number; y: number; tx: number; ty: number } | null = null;

  constructor(svg: SVGSVGElement, handlers: RenderHandlers) {
    this.svg = svg;
    this.handlers = handlers;
    this.svg.innerHTML = "";
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
  }

  setSelection(name: string | null) {
    this.selected = name;
    this.paint();
  }

  getTransform(): ViewTransform {
    return { ...this.transform };
  }

  fitToContent() {
    if (!this.diagram || this.diagram.entities.length === 0) return;
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const e of this.diagram.entities) {
      const p = e.position ?? { x: 0, y: 0 };
      const h = cardHeight(e);
      minX = Math.min(minX, p.x);
      minY = Math.min(minY, p.y);
      maxX = Math.max(maxX, p.x + CARD_W);
      maxY = Math.max(maxY, p.y + h);
    }
    const vb = this.svg.getBoundingClientRect();
    const pad = 48;
    const w = maxX - minX || 1;
    const h = maxY - minY || 1;
    const kx = (vb.width - pad * 2) / w;
    const ky = (vb.height - pad * 2) / h;
    const k = Math.max(0.35, Math.min(1.4, Math.min(kx, ky)));
    this.transform.k = k;
    this.transform.x = (vb.width - w * k) / 2 - minX * k;
    this.transform.y = (vb.height - h * k) / 2 - minY * k;
    this.applyTransform();
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

    for (const rel of this.diagram.relationships) {
      const a = byName.get(rel.fromEntity);
      const b = byName.get(rel.toEntity);
      if (!a || !b) continue;
      this.drawRelationship(rel, a, b);
    }

    for (const entity of this.diagram.entities) {
      this.drawEntity(entity);
    }
  }

  private drawRelationship(rel: Relationship, a: Entity, b: Entity) {
    const ca = entityCenter(a);
    const cb = entityCenter(b);
    const start = anchorPoint(a, cb);
    const end = anchorPoint(b, ca);

    // Orthogonal elbow
    const midX = (start.x + end.x) / 2;
    const path = `M ${start.x} ${start.y} L ${midX} ${start.y} L ${midX} ${end.y} L ${end.x} ${end.y}`;

    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    g.setAttribute("class", "relationship");

    const line = document.createElementNS("http://www.w3.org/2000/svg", "path");
    line.setAttribute("d", path);
    line.setAttribute("class", "rel-line");
    g.appendChild(line);

    const m1 = document.createElementNS("http://www.w3.org/2000/svg", "path");
    m1.setAttribute(
      "d",
      cardinalityPath(rel.fromCardinality, "start", start.x, start.y, end.x - start.x, end.y - start.y),
    );
    m1.setAttribute("class", "rel-marker");
    g.appendChild(m1);

    const m2 = document.createElementNS("http://www.w3.org/2000/svg", "path");
    m2.setAttribute(
      "d",
      cardinalityPath(rel.toCardinality, "end", end.x, end.y, start.x - end.x, start.y - end.y),
    );
    m2.setAttribute("class", "rel-marker");
    g.appendChild(m2);

    if (rel.label) {
      const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
      label.textContent = rel.label;
      label.setAttribute("x", String(midX));
      label.setAttribute("y", String((start.y + end.y) / 2 - 6));
      label.setAttribute("class", "rel-label");
      g.appendChild(label);
    }

    this.edges.appendChild(g);
  }

  private drawEntity(entity: Entity) {
    const pos = entity.position ?? { x: 0, y: 0 };
    const h = cardHeight(entity);
    const g = document.createElementNS("http://www.w3.org/2000/svg", "g");
    g.setAttribute("class", `entity-card${this.selected === entity.name ? " selected" : ""}`);
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
    // square off header bottom
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
      : [{ name: "—", dataType: "", isPk: false, isFk: false, isUnique: false, isNullable: true }];

    attrs.forEach((attr, i) => {
      const y = HEADER_H + 16 + i * ATTR_H;
      const name = document.createElementNS("http://www.w3.org/2000/svg", "text");
      name.setAttribute("x", String(PAD_X));
      name.setAttribute("y", String(y));
      name.setAttribute("class", `attr-name${attr.isPk ? " pk" : ""}`);
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
      (ev.target as Element).setPointerCapture?.(ev.pointerId);
      this.paint();
    });

    this.nodes.appendChild(g);
  }

  private onPointerDown = (ev: PointerEvent) => {
    if (ev.button !== 0) return;
    // empty canvas -> deselect / pan
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
      const x = this.dragging.ox + dx;
      const y = this.dragging.oy + dy;
      this.handlers.onMoveEntity(this.dragging.name, x, y);
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
  };

  private onWheel = (ev: WheelEvent) => {
    ev.preventDefault();
    if (ev.ctrlKey || ev.metaKey || ev.altKey) {
      const rect = this.svg.getBoundingClientRect();
      const mx = ev.clientX - rect.left;
      const my = ev.clientY - rect.top;
      const oldK = this.transform.k;
      const next = Math.min(2.5, Math.max(0.3, oldK * (ev.deltaY < 0 ? 1.08 : 0.92)));
      const wx = (mx - this.transform.x) / oldK;
      const wy = (my - this.transform.y) / oldK;
      this.transform.k = next;
      this.transform.x = mx - wx * next;
      this.transform.y = my - wy * next;
      this.applyTransform();
    } else {
      this.transform.x -= ev.deltaX;
      this.transform.y -= ev.deltaY;
      this.applyTransform();
    }
  };
}
