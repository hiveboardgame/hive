use crate::position::Position;

#[derive(PartialEq, Clone, Debug)]
pub struct SvgPosition {
    pub pos: (f32, f32),
    pub size: f32,
}

impl SvgPosition {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            pos: (x, y),
            size: 51.3,
        }
    }

    pub fn center(&self) -> (f32, f32) {
        let p = self.pos;
        let h = 2.0 * self.size;
        let w = 3.0_f32.sqrt() * self.size;
        if (p.1 as i32).rem_euclid(2) == 0 {
            (p.0 * w, p.1 * 0.75 * h) // even
        } else {
            (0.5 * w + p.0 * w, p.1 * 0.75 * h) // odd
        }
    }

    pub fn center_for_level(position: Position, level: usize, straight: bool) -> (f32, f32) {
        let position = Self::new(
            (position.q + position.r.div_euclid(2)) as f32,
            position.r as f32,
        );
        position.center_with_offset(SvgPosition::center_offset(level, straight))
    }

    pub fn center_offset(i: usize, straight: bool) -> (f32, f32) {
        if straight {
            (0.0, -9.0 * i as f32)
        } else {
            (-2.5 * i as f32, -3.5 * i as f32)
        }
    }

    pub fn center_with_offset(&self, center_offset: (f32, f32)) -> (f32, f32) {
        let center = self.center();
        (center.0 + center_offset.0, center.1 + center_offset.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property the renderer needs, stated directly: a hive drawn anywhere on the plane has
    /// to look like the same hive drawn at spawn. Every cell must therefore move by the same
    /// pixel amount under the same coordinate shift. Truncating the row halving breaks this
    /// either side of zero and tears the hive apart on screen.
    #[test]
    fn the_pixel_mapping_is_translation_equivariant() {
        for dq in -8..=8 {
            for dr in -8..=8 {
                let mut expected: Option<(f32, f32)> = None;
                for q in -64..64 {
                    for r in -64..64 {
                        let at = Position::new(q, r);
                        let moved = Position::new(q + dq, r + dr);
                        let (ax, ay) = SvgPosition::center_for_level(at, 0, false);
                        let (bx, by) = SvgPosition::center_for_level(moved, 0, false);
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
}
