use hive_lib::Position;

#[derive(PartialEq, Clone, Debug)]
pub struct SvgPos {
    pub pos: (f32, f32),
    pub size: f32,
}

impl SvgPos {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            pos: (x as f32, y as f32),
            size: 31.5,
        }
    }

    pub fn center_for_level(position: Position, level: usize, straight: bool) -> (f32, f32) {
        let position = Self {
            pos: (
                (position.q + position.r.div_euclid(2)) as f32,
                position.r as f32,
            ),
            size: 30.0,
        };
        position.center_with_offset(SvgPos::center_offset(level, straight))
    }

    pub fn center_offset(i: usize, straight: bool) -> (f32, f32) {
        if straight {
            (0.0, -6.0 * i as f32)
        } else {
            (-2.5 * i as f32, -3.5 * i as f32)
        }
    }

    pub fn center(&self) -> (f32, f32) {
        let p = self.pos;
        let h = 2.0 * self.size;
        let w = 3.0_f32.sqrt() * self.size;
        if (p.1 as i32).rem_euclid(2) == 0 {
            // even
            (p.0 * w, p.1 * 0.75 * h)
        } else {
            // odd
            (0.5 * w + p.0 * w, p.1 * 0.75 * h)
        }
    }

    pub fn center_with_offset(&self, center_offset: (f32, f32)) -> (f32, f32) {
        let center = self.center();
        (center.0 + center_offset.0, center.1 + center_offset.1)
    }
}

/// Hex size for the level-0 layout; must match `center_for_level`.
pub const HEX_SIZE: f32 = 30.0;

pub fn hex_width() -> f32 {
    3.0_f32.sqrt() * HEX_SIZE
}

pub fn hex_height() -> f32 {
    2.0 * HEX_SIZE
}

/// Inverse of `center` at level 0 (screen point → hex), for click hit-testing.
/// Mirrors `center`'s row parity and flooring `r` halving so it round-trips; stack
/// offsets only apply above level 0, so `straight` doesn't matter here.
pub fn position_from_svg(x: f32, y: f32) -> Position {
    let w = hex_width();
    let row_height = 0.75 * hex_height(); // vertical spacing between rows
    let r = (y / row_height).round() as i32;
    let base = if r.rem_euclid(2) == 0 { 0.0 } else { 0.5 * w };
    let q = ((x - base) / w).round() as i32 - r.div_euclid(2);
    Position::new(q, r)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Halving `r` has to floor, not truncate, or the `rem_euclid` parity correction disagrees
    /// with it for every odd negative row and lands the cell a full hex out. Nothing wraps the
    /// board into `0..BOARD_SIZE` any more, so a hive that drifts north gets there.
    #[test]
    fn adjacent_cells_stay_adjacent_north_of_the_origin() {
        let w = hex_width();
        for r in -8..8 {
            for q in -4..4 {
                let at = Position::new(q, r);
                for around in at.positions_around() {
                    let (ax, ay) = SvgPos::center_for_level(at, 0, false);
                    let (bx, by) = SvgPos::center_for_level(around, 0, false);
                    let distance = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
                    assert!(
                        (distance - w).abs() < 0.5,
                        "{at} and its neighbour {around} render {distance} apart, not {w}"
                    );
                }
            }
        }
    }

    /// The property the renderer needs, stated directly: a hive drawn anywhere on the plane has
    /// to look like the same hive drawn at spawn, so every cell must move by the same pixel
    /// amount under the same coordinate shift. Stronger than the adjacency check above - that
    /// only looks at neighbours, this pins the whole plane.
    #[test]
    fn the_pixel_mapping_is_translation_equivariant() {
        for dq in -8..=8 {
            for dr in -8..=8 {
                let mut expected: Option<(f32, f32)> = None;
                for q in -64..64 {
                    for r in -64..64 {
                        let at = Position::new(q, r);
                        let moved = Position::new(q + dq, r + dr);
                        let (ax, ay) = SvgPos::center_for_level(at, 0, false);
                        let (bx, by) = SvgPos::center_for_level(moved, 0, false);
                        let shift = (bx - ax, by - ay);
                        match expected {
                            None => expected = Some(shift),
                            Some(seen) => assert!(
                                (seen.0 - shift.0).abs() < 0.01 && (seen.1 - shift.1).abs() < 0.01,
                                "shift ({dq},{dr}) moves {at} by {shift:?}, others by {seen:?}"
                            ),
                        }
                    }
                }
            }
        }
    }

    /// Hit-testing has to invert `center_for_level` on the same rows.
    #[test]
    fn screen_points_round_trip_north_of_the_origin() {
        for r in -8..8 {
            for q in -4..4 {
                let at = Position::new(q, r);
                let (x, y) = SvgPos::center_for_level(at, 0, false);
                assert_eq!(position_from_svg(x, y), at, "{at} did not round trip");
            }
        }
    }
}
