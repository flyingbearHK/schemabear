//! Simple deterministic auto-layout for entity cards.

use crate::model::{Diagram, Position};

const CARD_WIDTH: f64 = 220.0;
const CARD_BASE_HEIGHT: f64 = 48.0;
const ATTR_HEIGHT: f64 = 22.0;
const GAP_X: f64 = 80.0;
const GAP_Y: f64 = 60.0;
const ORIGIN_X: f64 = 40.0;
const ORIGIN_Y: f64 = 40.0;
const COLS: usize = 3;

/// Assign grid positions to entities missing coordinates.
/// Existing positions are preserved unless `force` is true.
pub fn auto_layout(diagram: &mut Diagram, force: bool) {
    let mut idx = 0usize;
    for entity in &mut diagram.entities {
        if entity.position.is_some() && !force {
            continue;
        }
        let col = idx % COLS;
        let row = idx / COLS;
        let height = CARD_BASE_HEIGHT + entity.attributes.len() as f64 * ATTR_HEIGHT;
        // Stagger rows by max height in a simple way using fixed slot height.
        let slot_h = CARD_BASE_HEIGHT + 12.0 * ATTR_HEIGHT;
        entity.position = Some(Position {
            x: ORIGIN_X + col as f64 * (CARD_WIDTH + GAP_X),
            y: ORIGIN_Y + row as f64 * (slot_h + GAP_Y) + (height * 0.0),
        });
        idx += 1;
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
            let h = CARD_BASE_HEIGHT + entity.attributes.len() as f64 * ATTR_HEIGHT;
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
    use crate::model::{Attribute, Entity};

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
}
