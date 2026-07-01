use crate::{board::BOARD_SIZE, position::Position};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TorusArray<T>
where
    T: Clone,
{
    data: [T; (BOARD_SIZE * BOARD_SIZE) as usize],
    default: T,
}

impl<T> TorusArray<T>
where
    T: Clone,
{
    pub fn new(default: T) -> Self {
        Self {
            data: [(); (BOARD_SIZE * BOARD_SIZE) as usize].map(|_| default.clone()),
            default,
        }
    }

    #[inline(always)]
    pub fn get(&self, position: Position) -> &T {
        &self.data[(position.r * BOARD_SIZE + position.q) as usize]
    }

    #[inline(always)]
    pub fn get_mut(&mut self, position: Position) -> &mut T {
        &mut self.data[(position.r * BOARD_SIZE + position.q) as usize]
    }

    #[inline(always)]
    pub fn set(&mut self, position: Position, element: T) {
        self.data[(position.r * BOARD_SIZE + position.q) as usize] = element;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_new_insert_get() {
        let mut arr = TorusArray::new(0_i32);
        let position = Position::new(0, 1);
        arr.set(position, 1);
        assert_eq!(*arr.get(position), 1);
    }
}
