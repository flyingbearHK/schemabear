//! Deterministic auto-layout for entity cards.
//!
//! Uses a simple relationship-aware layered layout (left → right) so
//! parent/lookup tables sit upstream of dependent facts. Falls back to a
//! height-aware grid when the graph is empty or disconnected.

use crate::model::{Diagram, Position};
use std::collections::{HashMap, VecDeque};

const CARD_WIDTH: f64 = 240.0;
const CARD_BASE_HEIGHT: f64 = 44.0;
const ATTR_HEIGHT: f64 = 22.0;
const GAP_X: f64 = 100.0;
const GAP_Y: f64 = 48.0;
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
    let n = diagram.entities.len().max(1);
    let cols = ((n as f64).sqrt().ceil() as usize).clamp(2, 4);

    // First pass: heights per row slot.
    let mut heights: Vec<f64> = Vec::new();
    let mut idx = 0usize;
    for entity in &diagram.entities {
        if entity.position.is_some() && !force {
            continue;
        }
        let row = idx / cols;
        let h = card_height(entity.attributes.len());
        if heights.len() <= row {
            heights.resize(row + 1, 0.0);
        }
        heights[row] = heights[row].max(h);
        idx += 1;
    }

    let mut row_y = vec![ORIGIN_Y];
    for h in &heights {
        let prev = *row_y.last().unwrap_or(&ORIGIN_Y);
        row_y.push(prev + h + GAP_Y);
    }

    idx = 0;
    for entity in &mut diagram.entities {
        if entity.position.is_some() && !force {
            continue;
        }
        let col = idx % cols;
        let row = idx / cols;
        entity.position = Some(Position {
            x: ORIGIN_X + col as f64 * (CARD_WIDTH + GAP_X),
            y: row_y.get(row).copied().unwrap_or(ORIGIN_Y),
        });
        idx += 1;
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
        // Orient edge from "one" side toward "many" side when possible so
        // dimensions/lookups sit left of facts.
        let (src, dst) = prefer_one_to_many(a, b, rel.from_cardinality, rel.to_cardinality);
        if !outgoing[src].contains(&dst) {
            outgoing[src].push(dst);
            indegree[dst] += 1;
        }
    }

    // Kahn-style layering; cycles get appended at the end.
    let mut layer = vec![0usize; n];
    let mut q: VecDeque<usize> = VecDeque::new();
    for (i, &deg) in indegree.iter().enumerate() {
        if deg == 0 {
            q.push_back(i);
            layer[i] = 0;
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
        // Break cycles: place remaining nodes after max known layer.
        let base = layer.iter().copied().max().unwrap_or(0) + 1;
        let mut extra = 0usize;
        for i in 0..n {
            if indegree_work[i] > 0 {
                layer[i] = base + extra % 2;
                extra += 1;
            }
        }
    }

    // Group by layer.
    let max_layer = layer.iter().copied().max().unwrap_or(0);
    let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];
    for (i, &l) in layer.iter().enumerate() {
        buckets[l].push(i);
    }

    // Light ordering: entities with more attrs lower in the column.
    for bucket in &mut buckets {
        bucket.sort_by(|&a, &b| {
            let ha = diagram.entities[a].attributes.len();
            let hb = diagram.entities[b].attributes.len();
            ha.cmp(&hb).then_with(|| {
                diagram.entities[a]
                    .name
                    .cmp(&diagram.entities[b].name)
            })
        });
    }

    // Place columns left→right; stack vertically with real heights.
    for (l, bucket) in buckets.iter().enumerate() {
        let mut y = ORIGIN_Y;
        for &idx in bucket {
            let entity = &mut diagram.entities[idx];
            if entity.position.is_some() && !force {
                // Still advance y using existing box so neighbors don't pile on.
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

    // Any entity still missing a position (shouldn't happen) → grid tail.
    let missing = diagram.entities.iter().any(|e| e.position.is_none());
    if missing {
        grid_layout(diagram, force);
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
}
