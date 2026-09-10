use crate::{board::BOARD_SIZE, position::Position};

pub const SMALL_SIZE: i32 = 16;
/// Move generation reads at most two cells beyond the hive - a spawn candidate sits one out,
/// and the moves from it probe one further - so the window has to keep that much slack on
/// every side or a legal probe would read past the edge.
pub const MARGIN: i32 = 2;
/// Where a fresh board opens its window: centred on [`Position::initial_spawn_position`].
pub const INITIAL_ORIGIN: Position = Position {
    q: (BOARD_SIZE - SMALL_SIZE) / 2,
    r: (BOARD_SIZE - SMALL_SIZE) / 2,
};

/// Small is inline so the common clone is a flat memcpy; Big is boxed and rare.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Storage<T> {
    Small([T; (SMALL_SIZE * SMALL_SIZE) as usize]),
    Big(Box<[T; (BOARD_SIZE * BOARD_SIZE) as usize]>),
}

/// A fixed-size window onto the unbounded hex plane. Cells outside it read as the default and
/// are never written; [`Board::reframe`](crate::board::Board::reframe) slides the window to
/// follow the hive, which is why piece coordinates themselves never have to move.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowArray<T>
where
    T: Clone,
{
    storage: Storage<T>,
    default: T,
    origin: Position,
}

impl<T> WindowArray<T>
where
    T: Clone,
{
    pub fn new(default: T) -> Self {
        Self::framed(INITIAL_ORIGIN, true, default)
    }

    pub fn framed(origin: Position, small: bool, default: T) -> Self {
        let storage = if small {
            Storage::Small([(); (SMALL_SIZE * SMALL_SIZE) as usize].map(|_| default.clone()))
        } else {
            Storage::Big(Self::boxed_big(&default))
        };
        Self {
            storage,
            default,
            origin,
        }
    }

    /// Scratches must cover the same window as the board they walk.
    pub fn new_like<U: Clone>(other: &WindowArray<U>, default: T) -> Self {
        Self::framed(other.origin, other.is_small(), default)
    }

    fn boxed_big(default: &T) -> Box<[T; (BOARD_SIZE * BOARD_SIZE) as usize]> {
        vec![default.clone(); (BOARD_SIZE * BOARD_SIZE) as usize]
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| unreachable!("sized to BOARD_SIZE^2"))
    }

    pub fn origin(&self) -> Position {
        self.origin
    }

    pub fn size(&self) -> i32 {
        match &self.storage {
            Storage::Small(_) => SMALL_SIZE,
            Storage::Big(_) => BOARD_SIZE,
        }
    }

    pub fn contains(&self, position: Position) -> bool {
        (self.origin.q..self.origin.q + self.size()).contains(&position.q)
            && (self.origin.r..self.origin.r + self.size()).contains(&position.r)
    }

    fn index(&self, position: Position) -> usize {
        ((position.r - self.origin.r) * self.size() + (position.q - self.origin.q)) as usize
    }

    pub fn get(&self, position: Position) -> &T {
        if !self.contains(position) {
            // Writes are guarded to the window, so outside it is empty by construction.
            return &self.default;
        }
        let index = self.index(position);
        match &self.storage {
            Storage::Small(data) => &data[index],
            Storage::Big(data) => &data[index],
        }
    }

    pub fn get_mut(&mut self, position: Position) -> &mut T {
        debug_assert!(
            self.contains(position),
            "write outside the window: {position}"
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
        (self.size() * self.size()) as usize
    }

    /// Widen in place, keeping the old window centred in the new one so nothing already
    /// written ends up against an edge. One-way; [`Self::reframe`] is what shrinks.
    pub fn grow(&mut self) {
        let Storage::Small(small) = &self.storage else {
            return;
        };
        let shift = (BOARD_SIZE - SMALL_SIZE) / 2;
        let mut big = Self::boxed_big(&self.default);
        for r in 0..SMALL_SIZE {
            for q in 0..SMALL_SIZE {
                big[((r + shift) * BOARD_SIZE + q + shift) as usize] =
                    small[(r * SMALL_SIZE + q) as usize].clone();
            }
        }
        self.storage = Storage::Big(big);
        self.origin = Position {
            q: self.origin.q - shift,
            r: self.origin.r - shift,
        };
    }

    /// Move the window, carrying every cell that still falls inside it. Cells left behind are
    /// the caller's problem: the board only ever reframes around the whole hive.
    pub fn reframe(&mut self, origin: Position, small: bool) {
        let mut moved = Self::framed(origin, small, self.default.clone());
        for r in 0..self.size() {
            for q in 0..self.size() {
                let at = Position {
                    q: self.origin.q + q,
                    r: self.origin.r + r,
                };
                if moved.contains(at) {
                    let index = self.index(at);
                    let cell = match &self.storage {
                        Storage::Small(data) => data[index].clone(),
                        Storage::Big(data) => data[index].clone(),
                    };
                    moved.set(at, cell);
                }
            }
        }
        *self = moved;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_insert_get() {
        let mut arr = WindowArray::new(0_i32);
        let position = Position::new(8, 9);
        arr.set(position, 1);
        assert_eq!(*arr.get(position), 1);
    }

    /// The point of the small storage: cloning a board stays off the heap.
    #[test]
    fn small_storage_is_actually_small() {
        assert!(std::mem::size_of::<WindowArray<crate::bug_stack::BugStack>>() < 3 * 1024);
        assert!(std::mem::size_of::<WindowArray<u8>>() < 512);
    }

    #[test]
    fn grow_keeps_cells_at_their_coordinates() {
        let mut arr = WindowArray::new(0_i32);
        arr.set(Position::new(10, 20), 7);
        assert!(arr.is_small());
        arr.grow();
        assert!(!arr.is_small());
        assert_eq!(*arr.get(Position::new(10, 20)), 7);
        arr.set(Position::new(0, 31), 9);
        assert_eq!(*arr.get(Position::new(0, 31)), 9);
    }

    /// The window is what moves, so a cell keeps its coordinates across a reframe.
    #[test]
    fn reframe_keeps_cells_at_their_coordinates() {
        let mut arr = WindowArray::new(0_i32);
        arr.set(Position::new(20, 20), 5);
        arr.reframe(Position::new(14, 14), true);
        assert_eq!(arr.origin(), Position::new(14, 14));
        assert_eq!(*arr.get(Position::new(20, 20)), 5);
    }

    /// Nothing outside the window is readable, so a stale cell cannot resurface later.
    #[test]
    fn reframe_drops_cells_the_new_window_misses() {
        let mut arr = WindowArray::new(0_i32);
        arr.set(Position::new(9, 9), 5);
        arr.reframe(Position::new(20, 20), true);
        assert_eq!(*arr.get(Position::new(9, 9)), 0);
    }

    /// Coordinates run off in both directions once nothing wraps them.
    #[test]
    fn the_window_addresses_negative_coordinates() {
        let mut arr = WindowArray::new(0_i32);
        arr.reframe(Position::new(-8, -8), true);
        arr.set(Position::new(-3, -1), 4);
        assert_eq!(*arr.get(Position::new(-3, -1)), 4);
        assert_eq!(*arr.get(Position::new(16, 16)), 0);
    }
}
