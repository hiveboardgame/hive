use hive_lib::position::Position;

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

    pub fn center_for_level(position: Position, level: usize) -> (f32, f32) {
        let position = Self {
            pos: ((position.q + position.r / 2) as f32, position.r as f32),
            size: 31.5,
        };
        position.center_with_offset(SvgPos::center_offset(level))
    }

    pub fn center_offset(i: usize) -> (f32, f32) {
        (-1.5 * i as f32, -2.5 * i as f32)
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
