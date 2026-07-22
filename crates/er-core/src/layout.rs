//! Deterministic auto-layout for entity cards.
//!
//! Improved Sugiyama-style pipeline tuned for ER diagrams:
//! 1. Orient edges parent → child (1 → many when possible)
//! 2. Break cycles (greedy feedback arc set) so layering is stable
//! 3. Longest-path layer assignment (left → right)
//! 4. Crossing reduction: barycenter sweeps + adjacent transposition
//! 5. Y placement: neighbor-median attraction + overlap resolution
//!
//! This is **not** Mermaid’s layout — positions live in our model and the
//! SVG renderer draws orthogonal links between the placed cards.

use crate::model::{Cardinality, Diagram, Position};
use std::collections::{HashMap, HashSet, VecDeque};

const CARD_WIDTH: f64 = 240.0;
const CARD_BASE_HEIGHT: f64 = 44.0;
const ATTR_HEIGHT: f64 = 22.0;
/// Extra horizontal corridor so multi-hop edges can fan without stacking.
const GAP_X: f64 = 160.0;
const GAP_Y: f64 = 48.0;
const ORIGIN_X: f64 = 48.0;
const ORIGIN_Y: f64 = 48.0;
const BARYCENTER_SWEEPS: usize = 8;
const TRANSPOSE_SWEEPS: usize = 8;
const MEDIAN_SWEEPS: usize = 14;

fn card_height(attr_count: usize) -> f64 {
    CARD_BASE_HEIGHT + attr_count.max(1) as f64 * ATTR_HEIGHT + 8.0
}

/// Assign positions to entities.
/// Existing positions are preserved unless `force` is true.
pub fn auto_layout(diagram: &mut Diagram, force: bool) {
    if diagram.entities.is_empty() {
        return;
    }

    if !force && diagram.entities.iter().all(|e| e.position.is_some()) {
        return;
    }

    if diagram.relationships.is_empty() {
        grid_layout(diagram, force);
        return;
    }

    layered_layout(diagram, force);
}

fn grid_layout(diagram: &mut Diagram, force: bool) {
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

    let mut row_heights = vec![0.0f64; (n + cols - 1) / cols];
    for (slot, &idx) in pending.iter().enumerate() {
        let row = slot / cols;
        let h = card_height(diagram.entities[idx].attributes.len());
        row_heights[row] = row_heights[row].max(h);
    }

    let mut row_y = vec![ORIGIN_Y; row_heights.len()];
    for r in 1..row_heights.len() {
        row_y[r] = row_y[r - 1] + row_heights[r - 1] + GAP_Y;
    }

    for (slot, &idx) in pending.iter().enumerate() {
        let col = slot % cols;
        let row = slot / cols;
        diagram.entities[idx].position = Some(Position {
            x: ORIGIN_X + col as f64 * (CARD_WIDTH + GAP_X),
            y: row_y[row],
        });
    }
}

fn layered_layout(diagram: &mut Diagram, force: bool) {
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

    // --- Build oriented unique edges (parent → child when cardinality allows) ---
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
        // Keep one directed edge per undirected pair for layering.
        if seen_pairs.insert(key) {
            directed.push((src, dst));
        }
    }

    // Degree for hub-aware initial order.
    let mut degree = vec![0usize; n];
    for &(u, v) in &directed {
        degree[u] += 1;
        degree[v] += 1;
    }

    // --- Greedy feedback arc set ---
    let dag_edges = break_cycles(n, &directed);

    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut incoming: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut indegree = vec![0usize; n];
    for &(u, v) in &dag_edges {
        if !outgoing[u].contains(&v) {
            outgoing[u].push(v);
            incoming[v].push(u);
            indegree[v] += 1;
        }
    }

    // Undirected adjacency for median Y (all original relationships).
    let mut undirected: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(u, v) in &directed {
        if !undirected[u].contains(&v) {
            undirected[u].push(v);
            undirected[v].push(u);
        }
    }

    // Undirected edge list used for crossing counts (layer-agnostic endpoints).
    let undirected_edges: Vec<(usize, usize)> = directed
        .iter()
        .map(|&(u, v)| if u < v { (u, v) } else { (v, u) })
        .collect();

    // --- Longest-path layering ---
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

    // Residual / isolated nodes.
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

    // Expand long edges with virtual nodes so crossing reduction sees every hop.
    // Real nodes: 0..n-1. Virtuals: n..
    let mut v_layer = layer.clone();
    let mut v_undirected: Vec<Vec<usize>> = undirected.clone();
    let mut v_edges: Vec<(usize, usize)> = undirected_edges.clone();
    let mut next_id = n;

    // Work on a copy of directed DAG edges spanning >1 layer.
    let long_edges: Vec<(usize, usize)> = dag_edges
        .iter()
        .copied()
        .filter(|&(u, v)| {
            let lu = layer[u];
            let lv = layer[v];
            lu.abs_diff(lv) > 1
        })
        .collect();

    for (u, v) in long_edges {
        let lu = layer[u];
        let lv = layer[v];
        // Normalize so left is the smaller layer endpoint.
        let (left, right, l_left, l_right) = if lu < lv {
            (u, v, lu, lv)
        } else {
            (v, u, lv, lu)
        };

        // Remove direct undirected edge; replace with a virtual chain through middle layers.
        v_undirected[left].retain(|&x| x != right);
        v_undirected[right].retain(|&x| x != left);
        v_edges.retain(|&(a, b)| {
            !((a == left && b == right) || (a == right && b == left))
        });

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

    // Virtual degree for stable sort (reals keep original degree, virtuals = 2).
    let mut v_degree = vec![2usize; v_n];
    for i in 0..n {
        v_degree[i] = degree[i];
    }

    // Initial order: high degree first, then id/name.
    for bucket in &mut buckets {
        bucket.sort_by(|&a, &b| {
            v_degree[b].cmp(&v_degree[a]).then_with(|| {
                if a < n && b < n {
                    diagram.entities[a].name.cmp(&diagram.entities[b].name)
                } else {
                    a.cmp(&b)
                }
            })
        });
    }

    // --- Crossing reduction: barycenter ---
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

    // Adjacent transposition on expanded graph.
    for _ in 0..TRANSPOSE_SWEEPS {
        let mut improved = false;
        for l in 0..max_layer {
            improved |= transpose_pair(l, &mut buckets, &v_edges);
        }
        if !improved {
            break;
        }
    }

    // Drop virtuals from buckets (ordering of reals is what we keep).
    for bucket in &mut buckets {
        bucket.retain(|&id| id < n);
    }

    // --- Coordinate assignment (real nodes only) ---
    let heights: Vec<f64> = (0..n)
        .map(|i| card_height(diagram.entities[i].attributes.len()))
        .collect();

    let mut y_pos = vec![0.0f64; n];
    for bucket in &buckets {
        let mut y = ORIGIN_Y;
        for &idx in bucket {
            y_pos[idx] = y;
            y += heights[idx] + GAP_Y;
        }
    }

    // Neighbor-median attraction (Brandes–Köpf inspired) on real undirected graph.
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
                .map(|&nb| y_pos[nb] + heights[nb] / 2.0)
                .collect();
            centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = if centers.len() % 2 == 1 {
                centers[centers.len() / 2]
            } else {
                let mid = centers.len() / 2;
                (centers[mid - 1] + centers[mid]) / 2.0
            };
            let target = median - heights[idx] / 2.0;
            y_pos[idx] = y_pos[idx] * 0.3 + target * 0.7;
        }

        // Resolve overlaps within each layer, preserving crossing-minimized order.
        for bucket in &buckets {
            if bucket.is_empty() {
                continue;
            }
            let mut cursor = f64::NEG_INFINITY;
            for &idx in bucket {
                let min_y = if cursor.is_finite() {
                    cursor + GAP_Y
                } else {
                    f64::NEG_INFINITY
                };
                if y_pos[idx] < min_y {
                    y_pos[idx] = min_y;
                }
                cursor = y_pos[idx] + heights[idx];
            }
        }
    }

    // Normalize min Y.
    let min_y = y_pos.iter().copied().fold(f64::INFINITY, f64::min);
    if min_y.is_finite() {
        let shift = ORIGIN_Y - min_y;
        if shift.abs() > 0.01 {
            for y in &mut y_pos {
                *y += shift;
            }
        }
    }

    for (i, entity) in diagram.entities.iter_mut().enumerate() {
        if entity.position.is_some() && !force {
            continue;
        }
        let l = layer[i];
        entity.position = Some(Position {
            x: ORIGIN_X + l as f64 * (CARD_WIDTH + GAP_X),
            y: y_pos[i],
        });
    }

    if diagram.entities.iter().any(|e| e.position.is_none()) {
        grid_layout(diagram, force);
    }
}

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
        if layer_of[nb] == neighbor_layer {
            if let Some(&r) = rank_of.get(&nb) {
                sum += r;
                count += 1.0;
            }
        }
    }
    if count == 0.0 {
        // Stay put relative to peers without pulling to an extreme.
        rank_of.values().sum::<f64>() / rank_of.len().max(1) as f64
    } else {
        sum / count
    }
}

/// Greedy cycle break: DFS; drop back-edges from the DAG used for layering.
fn break_cycles(n: usize, edges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(u, v) in edges {
        adj[u].push(v);
    }

    let mut state = vec![0u8; n]; // 0=unseen, 1=open, 2=done
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

/// Count crossings between two ordered layers for undirected edges that span them.
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
                continue; // same left node — not a crossing
            }
            if pairs[i].1 > pairs[j].1 {
                crossings += 1;
            }
        }
    }
    crossings
}

/// Bubble adjacent swaps on both layers of a consecutive pair to cut crossings.
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

    // Swap within right layer.
    let n_right = buckets[right_layer].len();
    if n_right >= 2 {
        loop {
            let mut swapped = false;
            for i in 0..n_right - 1 {
                let before = count_crossings(
                    &buckets[left_layer],
                    &buckets[right_layer],
                    edges,
                );
                buckets[right_layer].swap(i, i + 1);
                let after = count_crossings(
                    &buckets[left_layer],
                    &buckets[right_layer],
                    edges,
                );
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

    // Swap within left layer.
    let n_left = buckets[left_layer].len();
    if n_left >= 2 {
        loop {
            let mut swapped = false;
            for i in 0..n_left - 1 {
                let before = count_crossings(
                    &buckets[left_layer],
                    &buckets[right_layer],
                    edges,
                );
                buckets[left_layer].swap(i, i + 1);
                let after = count_crossings(
                    &buckets[left_layer],
                    &buckets[right_layer],
                    edges,
                );
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

/// Compute a loose bounding box for the diagram content.
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

/// Count straight-line segment crossings between entity centers (layout quality metric).
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
        assert_ne!(
            d.entities[0].position.unwrap().x,
            d.entities[1].position.unwrap().x
        );
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
        auto_layout(&mut d, true);
        let p = d.entity_by_name("PARENT").unwrap().position.unwrap();
        let c = d.entity_by_name("CHILD").unwrap().position.unwrap();
        assert!(p.x < c.x, "parent should be left of child: {p:?} vs {c:?}");
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
        auto_layout(&mut d, true);
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
        auto_layout(&mut d, true);
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
            crossings <= 5,
            "expected ≤5 center-line crossings on sample, got {crossings}"
        );
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
        auto_layout(&mut d, true);
        let hx = d.entity_by_name("HUB").unwrap().position.unwrap().x;
        for name in ["C1", "C2", "C3", "C4"] {
            let cx = d.entity_by_name(name).unwrap().position.unwrap().x;
            assert!(hx < cx, "hub left of {name}");
        }
    }
}
