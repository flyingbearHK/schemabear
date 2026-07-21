//! Deterministic auto-layout for entity cards.
//!
//! Uses a relationship-aware layered layout (left → right) so parent/lookup
//! tables sit upstream of dependent facts, with a barycenter pass to keep
//! connected entities aligned and reduce crossed relationship lines.

use crate::model::{Diagram, Position};
use std::collections::{HashMap, VecDeque};

const CARD_WIDTH: f64 = 240.0;
const CARD_BASE_HEIGHT: f64 = 44.0;
const ATTR_HEIGHT: f64 = 22.0;
const GAP_X: f64 = 120.0;
const GAP_Y: f64 = 56.0;
const ORIGIN_X: f64 = 48.0;
const ORIGIN_Y: f64 = 48.0;

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
    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut incoming: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut indegree = vec![0usize; n];

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
        if !outgoing[src].contains(&dst) {
            outgoing[src].push(dst);
            incoming[dst].push(src);
            indegree[dst] += 1;
        }
    }

    // Kahn-style longest-path layering.
    let mut layer = vec![0usize; n];
    let mut q: VecDeque<usize> = VecDeque::new();
    for (i, &deg) in indegree.iter().enumerate() {
        if deg == 0 {
            q.push_back(i);
        }
    }

    let mut seen = 0usize;
    let mut indegree_work = indegree.clone();
    while let Some(u) = q.pop_front() {
        seen += 1;
        for &v in &outgoing[u] {
            layer[v] = layer[v].max(layer[u] + 1);
            indegree_work[v] = indegree_work[v].saturating_sub(1);
            if indegree_work[v] == 0 {
                q.push_back(v);
            }
        }
    }

    if seen < n {
        let base = layer.iter().copied().max().unwrap_or(0) + 1;
        let mut extra = 0usize;
        for i in 0..n {
            if indegree_work[i] > 0 {
                layer[i] = base + extra % 2;
                extra += 1;
            }
        }
    }

    let max_layer = layer.iter().copied().max().unwrap_or(0);
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];
    for (i, &l) in layer.iter().enumerate() {
        buckets[l].push(i);
    }

    // Initial stable order.
    for bucket in &mut buckets {
        bucket.sort_by(|&a, &b| {
            diagram.entities[a]
                .name
                .cmp(&diagram.entities[b].name)
                .then_with(|| {
                    diagram.entities[a]
                        .attributes
                        .len()
                        .cmp(&diagram.entities[b].attributes.len())
                })
        });
    }

    // Barycenter sweeps to align connected entities and cut crossings.
    for _ in 0..4 {
        // Left → right using predecessors.
        for l in 1..=max_layer {
            let prev_pos: HashMap<usize, f64> = buckets[l - 1]
                .iter()
                .enumerate()
                .map(|(rank, &id)| (id, rank as f64))
                .collect();
            buckets[l].sort_by(|&a, &b| {
                barycenter(&incoming[a], &prev_pos)
                    .partial_cmp(&barycenter(&incoming[b], &prev_pos))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| diagram.entities[a].name.cmp(&diagram.entities[b].name))
            });
        }
        // Right → left using successors.
        if max_layer == 0 {
            break;
        }
        for l in (0..max_layer).rev() {
            let next_pos: HashMap<usize, f64> = buckets[l + 1]
                .iter()
                .enumerate()
                .map(|(rank, &id)| (id, rank as f64))
                .collect();
            buckets[l].sort_by(|&a, &b| {
                barycenter(&outgoing[a], &next_pos)
                    .partial_cmp(&barycenter(&outgoing[b], &next_pos))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| diagram.entities[a].name.cmp(&diagram.entities[b].name))
            });
        }
    }

    // Place columns left→right; center shorter columns vertically against the tallest.
    let mut col_heights = vec![0.0f64; buckets.len()];
    for (l, bucket) in buckets.iter().enumerate() {
        let mut h = 0.0;
        for (i, &idx) in bucket.iter().enumerate() {
            h += card_height(diagram.entities[idx].attributes.len());
            if i + 1 < bucket.len() {
                h += GAP_Y;
            }
        }
        col_heights[l] = h;
    }
    let max_h = col_heights.iter().copied().fold(0.0, f64::max);

    for (l, bucket) in buckets.iter().enumerate() {
        let mut y = ORIGIN_Y + ((max_h - col_heights[l]) / 2.0).max(0.0);
        for &idx in bucket {
            let entity = &mut diagram.entities[idx];
            if entity.position.is_some() && !force {
                if let Some(pos) = entity.position {
                    y = y.max(pos.y + card_height(entity.attributes.len()) + GAP_Y);
                }
                continue;
            }
            entity.position = Some(Position {
                x: ORIGIN_X + l as f64 * (CARD_WIDTH + GAP_X),
                y,
            });
            y += card_height(entity.attributes.len()) + GAP_Y;
        }
    }

    if diagram.entities.iter().any(|e| e.position.is_none()) {
        grid_layout(diagram, force);
    }
}

fn barycenter(neighbors: &[usize], rank_of: &HashMap<usize, f64>) -> f64 {
    let mut sum = 0.0;
    let mut count = 0.0;
    for &n in neighbors {
        if let Some(&r) = rank_of.get(&n) {
            sum += r;
            count += 1.0;
        }
    }
    if count == 0.0 {
        f64::MAX / 4.0
    } else {
        sum / count
    }
}

fn prefer_one_to_many(
    a: usize,
    b: usize,
    from_card: crate::model::Cardinality,
    to_card: crate::model::Cardinality,
) -> (usize, usize) {
    use crate::model::Cardinality::*;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Attribute, Cardinality, Entity, Relationship};

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
        // Noise entity with no edges should not sit between the chain on x.
        auto_layout(&mut d, true);
        let ax = d.entity_by_name("A").unwrap().position.unwrap().x;
        let bx = d.entity_by_name("B").unwrap().position.unwrap().x;
        let cx = d.entity_by_name("C").unwrap().position.unwrap().x;
        assert!(ax < bx && bx < cx);
    }
}
