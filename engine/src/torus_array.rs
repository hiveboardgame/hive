use crate::{board::BOARD_SIZE, position::Position};

pub const SMALL_SIZE: i32 = 16;
/// The small window sits centred on the board: `[8, 24)` on both axes.
pub const SMALL_OFFSET: i32 = (BOARD_SIZE - SMALL_SIZE) / 2;

/// Small is inline so the common clone is a flat memcpy; Big is boxed and rare.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Storage<T> {
    Small([T; (SMALL_SIZE * SMALL_SIZE) as usize]),
    Big(Box<[T; (BOARD_SIZE * BOARD_SIZE) as usize]>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TorusArray<T>
where
    T: Clone,
{
    storage: Storage<T>,
    default: T,
}

impl<T> TorusArray<T>
where
    T: Clone,
{
    pub fn new(default: T) -> Self {
        Self {
            storage: Storage::Small(
                [(); (SMALL_SIZE * SMALL_SIZE) as usize].map(|_| default.clone()),
            ),
            default,
        }
    }

    /// Scratches must cover the same window as the board they walk.
    pub fn new_like<U: Clone>(other: &TorusArray<U>, default: T) -> Self {
        let mut scratch = Self::new(default);
        if !other.is_small() {
            scratch.grow();
        }
        scratch
    }

    fn in_window(position: Position) -> bool {
        (SMALL_OFFSET..SMALL_OFFSET + SMALL_SIZE).contains(&position.q)
            && (SMALL_OFFSET..SMALL_OFFSET + SMALL_SIZE).contains(&position.r)
    }

    fn index(&self, position: Position) -> usize {
        match &self.storage {
            Storage::Small(_) => {
                ((position.r - SMALL_OFFSET).rem_euclid(SMALL_SIZE) * SMALL_SIZE
                    + (position.q - SMALL_OFFSET).rem_euclid(SMALL_SIZE)) as usize
            }
            Storage::Big(_) => (position.r * BOARD_SIZE + position.q) as usize,
        }
    }

    pub fn get(&self, position: Position) -> &T {
        match &self.storage {
            // Writes are guarded to the window, so outside it is empty by construction.
            Storage::Small(_) if !Self::in_window(position) => &self.default,
            Storage::Small(data) => &data[self.index(position)],
            Storage::Big(data) => &data[self.index(position)],
        }
    }

    pub fn get_mut(&mut self, position: Position) -> &mut T {
        debug_assert!(
            !self.is_small() || Self::in_window(position),
            "small-storage write outside the window: {position}"
        );
        let index = self.index(position);
        match &mut self.storage {
            Storage::Small(data) => &mut data[index],
            Storage::Big(data) => &mut data[index],
        }
    }

    pub fn set(&mut self, position: Position, element: T) {
        *self.get_mut(position) = element;
    }

    pub fn is_small(&self) -> bool {
        matches!(self.storage, Storage::Small(_))
    }

    pub fn cells(&self) -> usize {
        match &self.storage {
            Storage::Small(_) => (SMALL_SIZE * SMALL_SIZE) as usize,
            Storage::Big(_) => (BOARD_SIZE * BOARD_SIZE) as usize,
        }
    }

    /// One-way; shrinking happens by rebuilding in `Board::recenter`.
    pub fn grow(&mut self) {
        if let Storage::Small(small) = &self.storage {
            let mut big =
                vec![self.default.clone(); (BOARD_SIZE * BOARD_SIZE) as usize].into_boxed_slice();
            for r in 0..SMALL_SIZE {
                for q in 0..SMALL_SIZE {
                    big[((r + SMALL_OFFSET) * BOARD_SIZE + q + SMALL_OFFSET) as usize] =
                        small[(r * SMALL_SIZE + q) as usize].clone();
                }
            }
            let big: Box<[T; (BOARD_SIZE * BOARD_SIZE) as usize]> = big
                .try_into()
                .unwrap_or_else(|_| unreachable!("sized to BOARD_SIZE^2"));
            self.storage = Storage::Big(big);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_insert_get() {
        let mut arr = TorusArray::new(0_i32);
        let position = Position::new(8, 9);
        arr.set(position, 1);
        assert_eq!(*arr.get(position), 1);
    }

    /// The point of the small storage: cloning a board stays off the heap.
    #[test]
    fn small_storage_is_actually_small() {
        assert!(std::mem::size_of::<TorusArray<crate::bug_stack::BugStack>>() < 3 * 1024);
        assert!(std::mem::size_of::<TorusArray<u8>>() < 512);
    }

    #[test]
    fn grow_keeps_cells_at_their_coordinates() {
        let mut arr = TorusArray::new(0_i32);
        arr.set(Position::new(10, 20), 7);
        assert!(arr.is_small());
        arr.grow();
        assert!(!arr.is_small());
        assert_eq!(*arr.get(Position::new(10, 20)), 7);
        arr.set(Position::new(0, 31), 9);
        assert_eq!(*arr.get(Position::new(0, 31)), 9);
    }
}
