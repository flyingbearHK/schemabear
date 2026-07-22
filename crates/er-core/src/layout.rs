//! Deterministic auto-layout for entity cards.
//!
//! Sugiyama-style pipeline tuned for ER diagrams, plus optional force polish:
//! 1. Orient edges parent → child (1 → many when possible)
//! 2. Break cycles (greedy feedback arc set)
//! 3. Longest-path layer assignment
//! 4. Crossing reduction (barycenter + virtual nodes + adjacent transposition)
//! 5. Neighbor-median coordinate assignment
//! 6. Optional force-directed polish (repel cards, pull related tables)
//!
//! Supports **left→right** and **top→bottom** directions and three densities.

use crate::model::{Cardinality, Diagram, Position};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

const CARD_WIDTH: f64 = 240.0;
const CARD_BASE_HEIGHT: f64 = 44.0;
const ATTR_HEIGHT: f64 = 22.0;
const ORIGIN_X: f64 = 48.0;
const ORIGIN_Y: f64 = 48.0;
const BARYCENTER_SWEEPS: usize = 8;
const TRANSPOSE_SWEEPS: usize = 8;
const MEDIAN_SWEEPS: usize = 14;
const POLISH_ITERS_DEFAULT: usize = 48;

// ---------------------------------------------------------------------------
// Public options
// ---------------------------------------------------------------------------

/// Primary flow of the layered layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LayoutDirection {
    /// Parents on the left, dependents on the right (default).
    #[default]
    LeftRight,
    /// Parents on top, dependents below — better for wide/shallow schemas.
    TopBottom,
}

/// Spacing preset between cards and layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LayoutDensity {
    Compact,
    #[default]
    Comfortable,
    Wide,
}

/// Options for [`auto_layout_with`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutOptions {
    /// Recompute positions even when entities already have them.
    #[serde(default = "default_true")]
    pub force: bool,
    #[serde(default)]
    pub direction: LayoutDirection,
    #[serde(default)]
    pub density: LayoutDensity,
    /// Run force-directed polish after layered placement.
    #[serde(default = "default_true")]
    pub polish: bool,
}

fn default_true() -> bool {
    true
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            force: true,
            direction: LayoutDirection::LeftRight,
            density: LayoutDensity::Comfortable,
            polish: true,
        }
    }
}

#[derive(Clone, Copy)]
struct Spacing {
    /// Gap between layers (major axis).
    gap_major: f64,
    /// Gap between cards in a layer (minor axis).
    gap_minor: f64,
    polish_iters: usize,
    spring_rest: f64,
    repel_pad: f64,
}

impl LayoutDensity {
    fn spacing(self) -> Spacing {
        match self {
            Self::Compact => Spacing {
                gap_major: 100.0,
                gap_minor: 28.0,
                polish_iters: 36,
                spring_rest: 280.0,
                repel_pad: 16.0,
            },
            Self::Comfortable => Spacing {
                gap_major: 160.0,
                gap_minor: 48.0,
                polish_iters: POLISH_ITERS_DEFAULT,
                spring_rest: 340.0,
                repel_pad: 28.0,
            },
            Self::Wide => Spacing {
                gap_major: 220.0,
                gap_minor: 72.0,
                polish_iters: 56,
                spring_rest: 420.0,
                repel_pad: 40.0,
            },
        }
    }
}

fn card_height(attr_count: usize) -> f64 {
    CARD_BASE_HEIGHT + attr_count.max(1) as f64 * ATTR_HEIGHT + 8.0
}

/// Assign positions with default options (`force` only).
pub fn auto_layout(diagram: &mut Diagram, force: bool) {
    auto_layout_with(
        diagram,
        LayoutOptions {
            force,
            ..LayoutOptions::default()
        },
    );
}

/// Assign positions with full layout options.
pub fn auto_layout_with(diagram: &mut Diagram, opts: LayoutOptions) {
    if diagram.entities.is_empty() {
        return;
    }

    if !opts.force && diagram.entities.iter().all(|e| e.position.is_some()) {
        return;
    }

    let spacing = opts.density.spacing();

    if diagram.relationships.is_empty() {
        grid_layout(diagram, opts.force, opts.direction, spacing);
        return;
    }

    layered_layout(diagram, opts.force, opts.direction, spacing);

    if opts.polish {
        force_polish(diagram, opts.direction, spacing);
    }
}

// ---------------------------------------------------------------------------
// Grid (no relationships)
// ---------------------------------------------------------------------------

fn grid_layout(
    diagram: &mut Diagram,
    force: bool,
    direction: LayoutDirection,
    spacing: Spacing,
) {
    let pending: Vec<usize> = diagram
        .entities
        .iter()
        .enumerate()
        .filter(|(_, e)| force || e.position.is_none())
        .map(|(i, _)| i)
        .collect();

    if pending.is_empty() {
        return;
    }

    let n = pending.len().max(1);
    let cols = ((n as f64).sqrt().ceil() as usize).clamp(2, 4);

    let mut cell_h = vec![0.0f64; (n + cols - 1) / cols];
    for (slot, &idx) in pending.iter().enumerate() {
        let row = slot / cols;
        let h = card_height(diagram.entities[idx].attributes.len());
        cell_h[row] = cell_h[row].max(h);
    }

    let mut row_y = vec![ORIGIN_Y; cell_h.len()];
    for r in 1..cell_h.len() {
        row_y[r] = row_y[r - 1] + cell_h[r - 1] + spacing.gap_minor;
    }

    for (slot, &idx) in pending.iter().enumerate() {
        let col = slot % cols;
        let row = slot / cols;
        let (x, y) = match direction {
            LayoutDirection::LeftRight => (
                ORIGIN_X + col as f64 * (CARD_WIDTH + spacing.gap_major * 0.7),
                row_y[row],
            ),
            LayoutDirection::TopBottom => (
                ORIGIN_X + col as f64 * (CARD_WIDTH + spacing.gap_minor),
                ORIGIN_Y + row as f64 * (cell_h.get(row).copied().unwrap_or(120.0) + spacing.gap_major * 0.7),
            ),
        };
        diagram.entities[idx].position = Some(Position { x, y });
    }
}

// ---------------------------------------------------------------------------
// Layered Sugiyama
// ---------------------------------------------------------------------------

fn layered_layout(
    diagram: &mut Diagram,
    force: bool,
    direction: LayoutDirection,
    spacing: Spacing,
) {
    let names: Vec<String> = diagram.entities.iter().map(|e| e.name.clone()).collect();
    let index: HashMap<String, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_ascii_lowercase(), i))
        .collect();

    let n = names.len();
    if n == 0 {
        return;
    }

    let mut directed: Vec<(usize, usize)> = Vec::new();
    let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();

    for rel in &diagram.relationships {
        let Some(&a) = index.get(&rel.from_entity.to_ascii_lowercase()) else {
            continue;
        };
        let Some(&b) = index.get(&rel.to_entity.to_ascii_lowercase()) else {
            continue;
        };
        if a == b {
            continue;
        }
        let (src, dst) = prefer_one_to_many(a, b, rel.from_cardinality, rel.to_cardinality);
        let key = if src < dst { (src, dst) } else { (dst, src) };
        if seen_pairs.insert(key) {
            directed.push((src, dst));
        }
    }

    let mut degree = vec![0usize; n];
    for &(u, v) in &directed {
        degree[u] += 1;
        degree[v] += 1;
    }

    let dag_edges = break_cycles(n, &directed);

    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut indegree = vec![0usize; n];
    for &(u, v) in &dag_edges {
        if !outgoing[u].contains(&v) {
            outgoing[u].push(v);
            indegree[v] += 1;
        }
    }

    let mut undirected: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(u, v) in &directed {
        if !undirected[u].contains(&v) {
            undirected[u].push(v);
            undirected[v].push(u);
        }
    }

    let undirected_edges: Vec<(usize, usize)> = directed
        .iter()
        .map(|&(u, v)| if u < v { (u, v) } else { (v, u) })
        .collect();

    // Longest-path layering
    let mut layer = vec![0usize; n];
    let mut q: VecDeque<usize> = VecDeque::new();
    for (i, &deg) in indegree.iter().enumerate() {
        if deg == 0 {
            q.push_back(i);
        }
    }

    let mut indegree_work = indegree.clone();
    let mut processed = vec![false; n];
    while let Some(u) = q.pop_front() {
        if processed[u] {
            continue;
        }
        processed[u] = true;
        for &v in &outgoing[u] {
            layer[v] = layer[v].max(layer[u] + 1);
            indegree_work[v] = indegree_work[v].saturating_sub(1);
            if indegree_work[v] == 0 {
                q.push_back(v);
            }
        }
    }

    let base = layer.iter().copied().max().unwrap_or(0);
    let mut extra = 0usize;
    for i in 0..n {
        if !processed[i] {
            if degree[i] == 0 {
                layer[i] = 0;
            } else {
                layer[i] = base + 1 + extra % 2;
                extra += 1;
            }
        } else if degree[i] == 0 {
            layer[i] = 0;
        }
    }

    let max_layer = layer.iter().copied().max().unwrap_or(0);

    // Virtual nodes for long edges
    let mut v_layer = layer.clone();
    let mut v_undirected: Vec<Vec<usize>> = undirected.clone();
    let mut v_edges: Vec<(usize, usize)> = undirected_edges.clone();
    let mut next_id = n;

    let long_edges: Vec<(usize, usize)> = dag_edges
        .iter()
        .copied()
        .filter(|&(u, v)| layer[u].abs_diff(layer[v]) > 1)
        .collect();

    for (u, v) in long_edges {
        let lu = layer[u];
        let lv = layer[v];
        let (left, right, l_left, l_right) = if lu < lv {
            (u, v, lu, lv)
        } else {
            (v, u, lv, lu)
        };

        v_undirected[left].retain(|&x| x != right);
        v_undirected[right].retain(|&x| x != left);
        v_edges.retain(|&(a, b)| !((a == left && b == right) || (a == right && b == left)));

        let link = |a: usize, b: usize, und: &mut Vec<Vec<usize>>, edges: &mut Vec<(usize, usize)>| {
            if !und[a].contains(&b) {
                und[a].push(b);
                und[b].push(a);
            }
            let e = if a < b { (a, b) } else { (b, a) };
            if !edges.contains(&e) {
                edges.push(e);
            }
        };

        let mut prev = left;
        for l in (l_left + 1)..l_right {
            let vid = next_id;
            next_id += 1;
            v_layer.push(l);
            v_undirected.push(Vec::new());
            link(prev, vid, &mut v_undirected, &mut v_edges);
            prev = vid;
        }
        link(prev, right, &mut v_undirected, &mut v_edges);
    }

    let v_n = next_id;
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];
    for i in 0..v_n {
        let l = v_layer[i];
        if l <= max_layer {
            buckets[l].push(i);
        }
    }

    let mut v_degree = vec![2usize; v_n];
    for i in 0..n {
        v_degree[i] = degree[i];
    }

    for bucket in &mut buckets {
        bucket.sort_by(|&a, &b| {
            v_degree[b].cmp(&v_degree[a]).then_with(|| a.cmp(&b))
        });
    }

    for _ in 0..BARYCENTER_SWEEPS {
        for l in 1..=max_layer {
            let prev_pos = rank_map(&buckets[l - 1]);
            buckets[l].sort_by(|&a, &b| {
                let ba = layer_barycenter(a, l - 1, &v_layer, &v_undirected, &prev_pos);
                let bb = layer_barycenter(b, l - 1, &v_layer, &v_undirected, &prev_pos);
                ba.partial_cmp(&bb)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| v_degree[b].cmp(&v_degree[a]))
                    .then_with(|| a.cmp(&b))
            });
        }
        if max_layer == 0 {
            break;
        }
        for l in (0..max_layer).rev() {
            let next_pos = rank_map(&buckets[l + 1]);
            buckets[l].sort_by(|&a, &b| {
                let ba = layer_barycenter(a, l + 1, &v_layer, &v_undirected, &next_pos);
                let bb = layer_barycenter(b, l + 1, &v_layer, &v_undirected, &next_pos);
                ba.partial_cmp(&bb)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| v_degree[b].cmp(&v_degree[a]))
                    .then_with(|| a.cmp(&b))
            });
        }
    }

    for _ in 0..TRANSPOSE_SWEEPS {
        let mut improved = false;
        for l in 0..max_layer {
            improved |= transpose_pair(l, &mut buckets, &v_edges);
        }
        if !improved {
            break;
        }
    }

    for bucket in &mut buckets {
        bucket.retain(|&id| id < n);
    }

    let heights: Vec<f64> = (0..n)
        .map(|i| card_height(diagram.entities[i].attributes.len()))
        .collect();

    // Secondary-axis positions (within layer)
    let mut secondary = vec![0.0f64; n];
    for bucket in &buckets {
        let mut cursor = match direction {
            LayoutDirection::LeftRight => ORIGIN_Y,
            LayoutDirection::TopBottom => ORIGIN_X,
        };
        for &idx in bucket {
            secondary[idx] = cursor;
            let step = match direction {
                LayoutDirection::LeftRight => heights[idx] + spacing.gap_minor,
                LayoutDirection::TopBottom => CARD_WIDTH + spacing.gap_minor,
            };
            cursor += step;
        }
    }

    // Median attraction on secondary axis
    for sweep in 0..MEDIAN_SWEEPS {
        let iter: Box<dyn Iterator<Item = usize>> = if sweep % 2 == 0 {
            Box::new(0..n)
        } else {
            Box::new((0..n).rev())
        };

        for idx in iter {
            if undirected[idx].is_empty() {
                continue;
            }
            let mut centers: Vec<f64> = undirected[idx]
                .iter()
                .map(|&nb| match direction {
                    LayoutDirection::LeftRight => secondary[nb] + heights[nb] / 2.0,
                    LayoutDirection::TopBottom => secondary[nb] + CARD_WIDTH / 2.0,
                })
                .collect();
            centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = if centers.len() % 2 == 1 {
                centers[centers.len() / 2]
            } else {
                let mid = centers.len() / 2;
                (centers[mid - 1] + centers[mid]) / 2.0
            };
            let half = match direction {
                LayoutDirection::LeftRight => heights[idx] / 2.0,
                LayoutDirection::TopBottom => CARD_WIDTH / 2.0,
            };
            let target = median - half;
            secondary[idx] = secondary[idx] * 0.3 + target * 0.7;
        }

        for bucket in &buckets {
            if bucket.is_empty() {
                continue;
            }
            let mut cursor = f64::NEG_INFINITY;
            for &idx in bucket {
                let size = match direction {
                    LayoutDirection::LeftRight => heights[idx],
                    LayoutDirection::TopBottom => CARD_WIDTH,
                };
                let min_s = if cursor.is_finite() {
                    cursor + spacing.gap_minor
                } else {
                    f64::NEG_INFINITY
                };
                if secondary[idx] < min_s {
                    secondary[idx] = min_s;
                }
                cursor = secondary[idx] + size;
            }
        }
    }

    let min_s = secondary.iter().copied().fold(f64::INFINITY, f64::min);
    if min_s.is_finite() {
        let origin = match direction {
            LayoutDirection::LeftRight => ORIGIN_Y,
            LayoutDirection::TopBottom => ORIGIN_X,
        };
        let shift = origin - min_s;
        if shift.abs() > 0.01 {
            for s in &mut secondary {
                *s += shift;
            }
        }
    }

    // Primary axis = layer coordinate
    let layer_major: Vec<f64> = match direction {
        LayoutDirection::LeftRight => (0..=max_layer)
            .map(|l| ORIGIN_X + l as f64 * (CARD_WIDTH + spacing.gap_major))
            .collect(),
        LayoutDirection::TopBottom => {
            // Each layer row height = max card height in that layer.
            let mut row_h = vec![0.0f64; max_layer + 1];
            for (i, &l) in layer.iter().enumerate() {
                row_h[l] = row_h[l].max(heights[i]);
            }
            let mut ys = vec![ORIGIN_Y; max_layer + 1];
            for l in 1..=max_layer {
                ys[l] = ys[l - 1] + row_h[l - 1] + spacing.gap_major;
            }
            ys
        }
    };

    for (i, entity) in diagram.entities.iter_mut().enumerate() {
        if entity.position.is_some() && !force {
            continue;
        }
        let l = layer[i];
        let pos = match direction {
            LayoutDirection::LeftRight => Position {
                x: layer_major[l],
                y: secondary[i],
            },
            LayoutDirection::TopBottom => Position {
                x: secondary[i],
                y: layer_major[l],
            },
        };
        entity.position = Some(pos);
    }

    if diagram.entities.iter().any(|e| e.position.is_none()) {
        grid_layout(diagram, force, direction, spacing);
    }
}

// ---------------------------------------------------------------------------
// Force-directed polish
// ---------------------------------------------------------------------------

fn force_polish(diagram: &mut Diagram, direction: LayoutDirection, spacing: Spacing) {
    let n = diagram.entities.len();
    if n < 2 {
        return;
    }

    let heights: Vec<f64> = diagram
        .entities
        .iter()
        .map(|e| card_height(e.attributes.len()))
        .collect();

    let mut xs: Vec<f64> = diagram
        .entities
        .iter()
        .map(|e| e.position.map(|p| p.x).unwrap_or(ORIGIN_X))
        .collect();
    let mut ys: Vec<f64> = diagram
        .entities
        .iter()
        .map(|e| e.position.map(|p| p.y).unwrap_or(ORIGIN_Y))
        .collect();

    // Anchor weakly to initial layer coordinate so polish doesn't scramble structure.
    let anchor_x = xs.clone();
    let anchor_y = ys.clone();

    let name_index: HashMap<String, usize> = diagram
        .entities
        .iter()
        .enumerate()
        .map(|(i, e)| (e.name.to_ascii_lowercase(), i))
        .collect();

    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut seen = HashSet::new();
    for rel in &diagram.relationships {
        let Some(&a) = name_index.get(&rel.from_entity.to_ascii_lowercase()) else {
            continue;
        };
        let Some(&b) = name_index.get(&rel.to_entity.to_ascii_lowercase()) else {
            continue;
        };
        if a == b {
            continue;
        }
        let key = if a < b { (a, b) } else { (b, a) };
        if seen.insert(key) {
            edges.push(key);
        }
    }

    let rest = spacing.spring_rest;
    let pad = spacing.repel_pad;

    for iter in 0..spacing.polish_iters {
        let mut fx = vec![0.0f64; n];
        let mut fy = vec![0.0f64; n];
        let cooling = 1.0 - (iter as f64 / spacing.polish_iters as f64) * 0.7;

        // Pairwise repulsion (AABB + soft padding).
        for i in 0..n {
            for j in (i + 1)..n {
                let ai = aabb(xs[i], ys[i], heights[i]);
                let aj = aabb(xs[j], ys[j], heights[j]);
                let (ox, oy, depth) = separation_vector(ai, aj, pad);
                if depth > 0.0 {
                    let force = (depth + 8.0) * 0.55 * cooling;
                    fx[i] -= ox * force;
                    fy[i] -= oy * force;
                    fx[j] += ox * force;
                    fy[j] += oy * force;
                } else {
                    // Soft long-range repulsion so clusters breathe.
                    let cx_i = xs[i] + CARD_WIDTH / 2.0;
                    let cy_i = ys[i] + heights[i] / 2.0;
                    let cx_j = xs[j] + CARD_WIDTH / 2.0;
                    let cy_j = ys[j] + heights[j] / 2.0;
                    let dx = cx_j - cx_i;
                    let dy = cy_j - cy_i;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                    if dist < rest * 1.6 {
                        let f = (rest * 0.15) / dist * cooling;
                        fx[i] -= dx / dist * f;
                        fy[i] -= dy / dist * f;
                        fx[j] += dx / dist * f;
                        fy[j] += dy / dist * f;
                    }
                }
            }
        }

        // Spring attraction along relationships.
        for &(a, b) in &edges {
            let cx_a = xs[a] + CARD_WIDTH / 2.0;
            let cy_a = ys[a] + heights[a] / 2.0;
            let cx_b = xs[b] + CARD_WIDTH / 2.0;
            let cy_b = ys[b] + heights[b] / 2.0;
            let dx = cx_b - cx_a;
            let dy = cy_b - cy_a;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let stretch = dist - rest;
            let f = stretch * 0.045 * cooling;
            let ux = dx / dist;
            let uy = dy / dist;
            fx[a] += ux * f;
            fy[a] += uy * f;
            fx[b] -= ux * f;
            fy[b] -= uy * f;
        }

        // Layer anchors: keep major-axis order mostly intact.
        let anchor_w = 0.08 * cooling;
        match direction {
            LayoutDirection::LeftRight => {
                for i in 0..n {
                    fx[i] += (anchor_x[i] - xs[i]) * anchor_w * 1.6;
                    fy[i] += (anchor_y[i] - ys[i]) * anchor_w * 0.35;
                }
            }
            LayoutDirection::TopBottom => {
                for i in 0..n {
                    fy[i] += (anchor_y[i] - ys[i]) * anchor_w * 1.6;
                    fx[i] += (anchor_x[i] - xs[i]) * anchor_w * 0.35;
                }
            }
        }

        // Integrate with damping / step cap.
        let step = 0.65 * cooling;
        for i in 0..n {
            let m = (fx[i] * fx[i] + fy[i] * fy[i]).sqrt();
            let cap = 40.0;
            let (dx, dy) = if m > cap {
                (fx[i] / m * cap, fy[i] / m * cap)
            } else {
                (fx[i], fy[i])
            };
            xs[i] += dx * step;
            ys[i] += dy * step;
        }
    }

    // Hard overlap resolution (sweep).
    for _ in 0..12 {
        let mut moved = false;
        for i in 0..n {
            for j in (i + 1)..n {
                let ai = aabb(xs[i], ys[i], heights[i]);
                let aj = aabb(xs[j], ys[j], heights[j]);
                let (ox, oy, depth) = separation_vector(ai, aj, pad * 0.5);
                if depth > 0.0 {
                    xs[i] -= ox * depth * 0.5;
                    ys[i] -= oy * depth * 0.5;
                    xs[j] += ox * depth * 0.5;
                    ys[j] += oy * depth * 0.5;
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }

    // Normalize origin.
    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
    if min_x.is_finite() && min_y.is_finite() {
        let sx = ORIGIN_X - min_x;
        let sy = ORIGIN_Y - min_y;
        for i in 0..n {
            xs[i] += sx;
            ys[i] += sy;
        }
    }

    for (i, entity) in diagram.entities.iter_mut().enumerate() {
        entity.position = Some(Position {
            x: xs[i],
            y: ys[i],
        });
    }
}

#[derive(Clone, Copy)]
struct Aabb {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

fn aabb(x: f64, y: f64, h: f64) -> Aabb {
    Aabb {
        x0: x,
        y0: y,
        x1: x + CARD_WIDTH,
        y1: y + h,
    }
}

/// Unit separation vector from a→b and penetration depth (0 if separated beyond pad).
fn separation_vector(a: Aabb, b: Aabb, pad: f64) -> (f64, f64, f64) {
    let ax0 = a.x0 - pad;
    let ay0 = a.y0 - pad;
    let ax1 = a.x1 + pad;
    let ay1 = a.y1 + pad;
    let bx0 = b.x0 - pad;
    let by0 = b.y0 - pad;
    let bx1 = b.x1 + pad;
    let by1 = b.y1 + pad;

    let cx_a = (a.x0 + a.x1) / 2.0;
    let cy_a = (a.y0 + a.y1) / 2.0;
    let cx_b = (b.x0 + b.x1) / 2.0;
    let cy_b = (b.y0 + b.y1) / 2.0;

    let overlap_x = (ax1.min(bx1) - ax0.max(bx0)).max(0.0);
    let overlap_y = (ay1.min(by1) - ay0.max(by0)).max(0.0);

    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return (0.0, 0.0, 0.0);
    }

    // Push along the smaller overlap axis.
    let dx = cx_b - cx_a;
    let dy = cy_b - cy_a;
    if overlap_x < overlap_y {
        let s = if dx >= 0.0 { 1.0 } else { -1.0 };
        (s, 0.0, overlap_x)
    } else {
        let s = if dy >= 0.0 { 1.0 } else { -1.0 };
        (0.0, s, overlap_y)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn rank_map(bucket: &[usize]) -> HashMap<usize, f64> {
    bucket
        .iter()
        .enumerate()
        .map(|(rank, &id)| (id, rank as f64))
        .collect()
}

fn layer_barycenter(
    node: usize,
    neighbor_layer: usize,
    layer_of: &[usize],
    undirected: &[Vec<usize>],
    rank_of: &HashMap<usize, f64>,
) -> f64 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for &nb in &undirected[node] {
        if layer_of.get(nb).copied() == Some(neighbor_layer) {
            if let Some(&r) = rank_of.get(&nb) {
                sum += r;
                count += 1.0;
            }
        }
    }
    if count == 0.0 {
        rank_of.values().sum::<f64>() / rank_of.len().max(1) as f64
    } else {
        sum / count
    }
}

fn break_cycles(n: usize, edges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(u, v) in edges {
        adj[u].push(v);
    }

    let mut state = vec![0u8; n];
    let mut back: HashSet<(usize, usize)> = HashSet::new();

    fn dfs(u: usize, adj: &[Vec<usize>], state: &mut [u8], back: &mut HashSet<(usize, usize)>) {
        state[u] = 1;
        for &v in &adj[u] {
            if state[v] == 0 {
                dfs(v, adj, state, back);
            } else if state[v] == 1 {
                back.insert((u, v));
            }
        }
        state[u] = 2;
    }

    for i in 0..n {
        if state[i] == 0 {
            dfs(i, &adj, &mut state, &mut back);
        }
    }

    edges
        .iter()
        .copied()
        .filter(|e| !back.contains(e))
        .collect()
}

fn count_crossings(left: &[usize], right: &[usize], edges: &[(usize, usize)]) -> usize {
    let mut li = HashMap::new();
    for (i, &n) in left.iter().enumerate() {
        li.insert(n, i);
    }
    let mut ri = HashMap::new();
    for (i, &n) in right.iter().enumerate() {
        ri.insert(n, i);
    }

    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for &(u, v) in edges {
        if let (Some(&a), Some(&b)) = (li.get(&u), ri.get(&v)) {
            pairs.push((a, b));
        } else if let (Some(&a), Some(&b)) = (li.get(&v), ri.get(&u)) {
            pairs.push((a, b));
        }
    }
    pairs.sort_unstable();

    let mut crossings = 0usize;
    for i in 0..pairs.len() {
        for j in (i + 1)..pairs.len() {
            if pairs[i].0 == pairs[j].0 {
                continue;
            }
            if pairs[i].1 > pairs[j].1 {
                crossings += 1;
            }
        }
    }
    crossings
}

fn transpose_pair(
    left_layer: usize,
    buckets: &mut [Vec<usize>],
    edges: &[(usize, usize)],
) -> bool {
    let right_layer = left_layer + 1;
    if right_layer >= buckets.len() {
        return false;
    }

    let mut improved = false;

    let n_right = buckets[right_layer].len();
    if n_right >= 2 {
        loop {
            let mut swapped = false;
            for i in 0..n_right - 1 {
                let before = count_crossings(&buckets[left_layer], &buckets[right_layer], edges);
                buckets[right_layer].swap(i, i + 1);
                let after = count_crossings(&buckets[left_layer], &buckets[right_layer], edges);
                if after < before {
                    swapped = true;
                    improved = true;
                } else {
                    buckets[right_layer].swap(i, i + 1);
                }
            }
            if !swapped {
                break;
            }
        }
    }

    let n_left = buckets[left_layer].len();
    if n_left >= 2 {
        loop {
            let mut swapped = false;
            for i in 0..n_left - 1 {
                let before = count_crossings(&buckets[left_layer], &buckets[right_layer], edges);
                buckets[left_layer].swap(i, i + 1);
                let after = count_crossings(&buckets[left_layer], &buckets[right_layer], edges);
                if after < before {
                    swapped = true;
                    improved = true;
                } else {
                    buckets[left_layer].swap(i, i + 1);
                }
            }
            if !swapped {
                break;
            }
        }
    }

    improved
}

fn prefer_one_to_many(
    a: usize,
    b: usize,
    from_card: Cardinality,
    to_card: Cardinality,
) -> (usize, usize) {
    use Cardinality::*;
    let from_many = matches!(from_card, ZeroOrMany | OneOrMany);
    let to_many = matches!(to_card, ZeroOrMany | OneOrMany);
    if !from_many && to_many {
        (a, b)
    } else if from_many && !to_many {
        (b, a)
    } else {
        (a, b)
    }
}

/// Loose bounding box of diagram content.
pub fn bounding_box(diagram: &Diagram) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    let mut any = false;

    for entity in &diagram.entities {
        if let Some(pos) = entity.position {
            any = true;
            let h = card_height(entity.attributes.len());
            min_x = min_x.min(pos.x);
            min_y = min_y.min(pos.y);
            max_x = max_x.max(pos.x + CARD_WIDTH);
            max_y = max_y.max(pos.y + h);
        }
    }

    if any {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

/// Count straight-line segment crossings between entity centers (quality metric).
#[cfg(test)]
pub fn count_center_crossings(diagram: &Diagram) -> usize {
    let centers: HashMap<&str, (f64, f64)> = diagram
        .entities
        .iter()
        .filter_map(|e| {
            let p = e.position?;
            let h = card_height(e.attributes.len());
            Some((e.name.as_str(), (p.x + CARD_WIDTH / 2.0, p.y + h / 2.0)))
        })
        .collect();

    let mut segs: Vec<((f64, f64), (f64, f64))> = Vec::new();
    for rel in &diagram.relationships {
        let Some(&a) = centers.get(rel.from_entity.as_str()) else {
            continue;
        };
        let Some(&b) = centers.get(rel.to_entity.as_str()) else {
            continue;
        };
        segs.push((a, b));
    }

    let mut c = 0usize;
    for i in 0..segs.len() {
        for j in (i + 1)..segs.len() {
            let (a, b) = segs[i];
            let (c1, d) = segs[j];
            if points_close(a, c1)
                || points_close(a, d)
                || points_close(b, c1)
                || points_close(b, d)
            {
                continue;
            }
            if segments_intersect(a, b, c1, d) {
                c += 1;
            }
        }
    }
    c
}

#[cfg(test)]
fn points_close(p: (f64, f64), q: (f64, f64)) -> bool {
    (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6
}

#[cfg(test)]
fn segments_intersect(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    fn orient(p: (f64, f64), q: (f64, f64), r: (f64, f64)) -> f64 {
        (q.1 - p.1) * (r.0 - q.0) - (q.0 - p.0) * (r.1 - q.1)
    }
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    o1 * o2 < 0.0 && o3 * o4 < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Attribute, Entity, Relationship};
    use crate::sample::load_infor_hms_sample;

    #[test]
    fn lays_out_missing_positions() {
        let mut d = Diagram::new("t");
        d.entities.push(Entity::new("A"));
        d.entities.push(Entity {
            attributes: vec![Attribute::new("id", "int").pk()],
            ..Entity::new("B")
        });
        auto_layout(&mut d, true);
        assert!(d.entities.iter().all(|e| e.position.is_some()));
    }

    #[test]
    fn layered_puts_one_side_left_of_many() {
        let mut d = Diagram::new("t");
        d.entities.push(Entity::new("PARENT"));
        d.entities.push(Entity::new("CHILD"));
        d.relationships.push(Relationship::new(
            "PARENT",
            "CHILD",
            Cardinality::One,
            Cardinality::ZeroOrMany,
        ));
        auto_layout_with(
            &mut d,
            LayoutOptions {
                force: true,
                direction: LayoutDirection::LeftRight,
                density: LayoutDensity::Comfortable,
                polish: false,
            },
        );
        let p = d.entity_by_name("PARENT").unwrap().position.unwrap();
        let c = d.entity_by_name("CHILD").unwrap().position.unwrap();
        assert!(p.x < c.x, "parent should be left of child: {p:?} vs {c:?}");
    }

    #[test]
    fn top_bottom_puts_parent_above_child() {
        let mut d = Diagram::new("t");
        d.entities.push(Entity::new("PARENT"));
        d.entities.push(Entity::new("CHILD"));
        d.relationships.push(Relationship::new(
            "PARENT",
            "CHILD",
            Cardinality::One,
            Cardinality::ZeroOrMany,
        ));
        auto_layout_with(
            &mut d,
            LayoutOptions {
                force: true,
                direction: LayoutDirection::TopBottom,
                density: LayoutDensity::Comfortable,
                polish: false,
            },
        );
        let p = d.entity_by_name("PARENT").unwrap().position.unwrap();
        let c = d.entity_by_name("CHILD").unwrap().position.unwrap();
        assert!(p.y < c.y, "parent should be above child: {p:?} vs {c:?}");
    }

    #[test]
    fn density_wide_spreads_more_than_compact() {
        let mut compact = Diagram::new("t");
        let mut wide = Diagram::new("t");
        for name in ["A", "B", "C"] {
            compact.entities.push(Entity::new(name));
            wide.entities.push(Entity::new(name));
        }
        compact.relationships.push(Relationship::new(
            "A",
            "B",
            Cardinality::One,
            Cardinality::ZeroOrMany,
        ));
        compact.relationships.push(Relationship::new(
            "B",
            "C",
            Cardinality::One,
            Cardinality::ZeroOrMany,
        ));
        wide.relationships = compact.relationships.clone();

        auto_layout_with(
            &mut compact,
            LayoutOptions {
                force: true,
                direction: LayoutDirection::LeftRight,
                density: LayoutDensity::Compact,
                polish: false,
            },
        );
        auto_layout_with(
            &mut wide,
            LayoutOptions {
                force: true,
                direction: LayoutDirection::LeftRight,
                density: LayoutDensity::Wide,
                polish: false,
            },
        );

        let cw = compact.entity_by_name("C").unwrap().position.unwrap().x
            - compact.entity_by_name("A").unwrap().position.unwrap().x;
        let ww = wide.entity_by_name("C").unwrap().position.unwrap().x
            - wide.entity_by_name("A").unwrap().position.unwrap().x;
        assert!(ww > cw, "wide ({ww}) should span more than compact ({cw})");
    }

    #[test]
    fn barycenter_keeps_chain_aligned() {
        let mut d = Diagram::new("t");
        for name in ["A", "B", "C", "D"] {
            d.entities.push(Entity::new(name));
        }
        d.relationships.push(Relationship::new(
            "A",
            "B",
            Cardinality::One,
            Cardinality::ZeroOrMany,
        ));
        d.relationships.push(Relationship::new(
            "B",
            "C",
            Cardinality::One,
            Cardinality::ZeroOrMany,
        ));
        auto_layout_with(
            &mut d,
            LayoutOptions {
                force: true,
                polish: false,
                ..LayoutOptions::default()
            },
        );
        let ax = d.entity_by_name("A").unwrap().position.unwrap().x;
        let bx = d.entity_by_name("B").unwrap().position.unwrap().x;
        let cx = d.entity_by_name("C").unwrap().position.unwrap().x;
        assert!(ax < bx && bx < cx);
    }

    #[test]
    fn chain_entities_share_similar_y() {
        let mut d = Diagram::new("t");
        for name in ["A", "B", "C"] {
            d.entities.push(Entity::new(name));
        }
        d.relationships.push(Relationship::new(
            "A",
            "B",
            Cardinality::One,
            Cardinality::ZeroOrMany,
        ));
        d.relationships.push(Relationship::new(
            "B",
            "C",
            Cardinality::One,
            Cardinality::ZeroOrMany,
        ));
        auto_layout_with(
            &mut d,
            LayoutOptions {
                force: true,
                polish: false,
                ..LayoutOptions::default()
            },
        );
        let ay = d.entity_by_name("A").unwrap().position.unwrap().y;
        let by = d.entity_by_name("B").unwrap().position.unwrap().y;
        let cy = d.entity_by_name("C").unwrap().position.unwrap().y;
        assert!(
            (ay - by).abs() < 80.0 && (by - cy).abs() < 80.0,
            "chain should stay roughly aligned: {ay}, {by}, {cy}"
        );
    }

    #[test]
    fn infor_sample_has_few_center_crossings() {
        let d = load_infor_hms_sample().expect("sample");
        let crossings = count_center_crossings(&d);
        eprintln!("infor sample center crossings = {crossings}");
        assert!(
            crossings <= 6,
            "expected ≤6 center-line crossings on sample, got {crossings}"
        );
    }

    #[test]
    fn polish_does_not_overlap_cards() {
        let mut d = load_infor_hms_sample().expect("sample");
        // Re-run with polish forced.
        auto_layout_with(&mut d, LayoutOptions::default());
        let n = d.entities.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let a = &d.entities[i];
                let b = &d.entities[j];
                let pa = a.position.unwrap();
                let pb = b.position.unwrap();
                let ha = card_height(a.attributes.len());
                let hb = card_height(b.attributes.len());
                let overlap_x = (pa.x + CARD_WIDTH).min(pb.x + CARD_WIDTH) - pa.x.max(pb.x);
                let overlap_y = (pa.y + ha).min(pb.y + hb) - pa.y.max(pb.y);
                assert!(
                    overlap_x <= 0.5 || overlap_y <= 0.5,
                    "cards {} and {} overlap",
                    a.name,
                    b.name
                );
            }
        }
    }

    #[test]
    fn star_hub_is_left_of_children() {
        let mut d = Diagram::new("t");
        d.entities.push(Entity::new("HUB"));
        for name in ["C1", "C2", "C3", "C4"] {
            d.entities.push(Entity::new(name));
            d.relationships.push(Relationship::new(
                "HUB",
                name,
                Cardinality::One,
                Cardinality::ZeroOrMany,
            ));
        }
        auto_layout_with(
            &mut d,
            LayoutOptions {
                force: true,
                polish: false,
                ..LayoutOptions::default()
            },
        );
        let hx = d.entity_by_name("HUB").unwrap().position.unwrap().x;
        for name in ["C1", "C2", "C3", "C4"] {
            let cx = d.entity_by_name(name).unwrap().position.unwrap().x;
            assert!(hx < cx, "hub left of {name}");
        }
    }
}
