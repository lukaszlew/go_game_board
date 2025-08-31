use crate::*;
pub use go_game_types::{Color, Player, Vertex};

pub const MAX_BOARD_SIZE: usize = 19;

// Base trait for natural number types
pub trait Nat: Copy + Clone + Eq + PartialEq + From<usize> + Into<usize> {
    const COUNT: usize;

    fn all() -> impl Iterator<Item = Self> {
        (0..Self::COUNT as usize).map(Self::from)
    }
}

// Implement Nat directly for go_game_types
impl Nat for Player {
    const COUNT: usize = Player::COUNT;
}

impl Nat for Color {
    const COUNT: usize = Color::COUNT;
}

impl Nat for Vertex {
    const COUNT: usize = Vertex::COUNT;
}

// Direction - local type that stays
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Dir {
    N = 0,
    E = 1,
    S = 2,
    W = 3,
    NW = 4,
    NE = 5,
    SE = 6,
    SW = 7,
}

impl Default for Dir {
    fn default() -> Self {
        Dir::N
    }
}

impl From<usize> for Dir {
    fn from(raw: usize) -> Self {
        match raw {
            0 => Dir::N,
            1 => Dir::E,
            2 => Dir::S,
            3 => Dir::W,
            4 => Dir::NW,
            5 => Dir::NE,
            6 => Dir::SE,
            7 => Dir::SW,
            _ => panic!("Invalid direction: {}", raw),
        }
    }
}

impl From<Dir> for usize {
    fn from(dir: Dir) -> usize {
        dir as usize
    }
}

impl Nat for Dir {
    const COUNT: usize = 8;
}

impl Dir {
    pub fn n() -> Self {
        Dir::N
    }
    pub fn e() -> Self {
        Dir::E
    }
    pub fn s() -> Self {
        Dir::S
    }
    pub fn w() -> Self {
        Dir::W
    }

    pub fn is_simple4(&self) -> bool {
        matches!(self, Dir::N | Dir::E | Dir::S | Dir::W)
    }

    pub fn opposite(&self) -> Self {
        match self {
            Dir::N => Dir::S,
            Dir::E => Dir::W,
            Dir::S => Dir::N,
            Dir::W => Dir::E,
            Dir::NW => Dir::SE,
            Dir::NE => Dir::SW,
            Dir::SE => Dir::NW,
            Dir::SW => Dir::NE,
        }
    }

    pub fn proximity(&self) -> usize {
        // 0 for direct neighbors (N,E,S,W), 1 for diagonal neighbors
        if self.is_simple4() {
            0
        } else {
            1
        }
    }
}

// Move - combines Player and Vertex
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Move {
    pub player: Player,
    pub vertex: Vertex,
}

impl Move {
    pub fn of_player_vertex(player: Player, vertex: Vertex) -> Self {
        Move { player, vertex }
    }
}

impl From<usize> for Move {
    fn from(raw: usize) -> Self {
        Move {
            player: Player::from(raw & 1),
            vertex: Vertex::from(raw >> 1),
        }
    }
}

impl From<Move> for usize {
    fn from(m: Move) -> usize {
        let player_raw: usize = m.player.into();
        let vertex_raw: usize = m.vertex.into();
        player_raw | (vertex_raw << 1)
    }
}

impl Nat for Move {
    // Move encoding: player | (vertex << 1)
    const COUNT: usize = Vertex::COUNT << 1;
}

// Helper function for Vertex creation with full coordinates (including sentinels)
pub fn vertex_of_coords_full(row: i32, column: i32) -> Vertex {
    assert!(row >= 0 && row < (MAX_BOARD_SIZE + 2) as i32);
    assert!(column >= 0 && column < (MAX_BOARD_SIZE + 2) as i32);
    // Adjust for 0-based internal coordinates
    Vertex::from_coords(row as isize - 1, column as isize - 1)
}

// Helper function for Vertex navigation
pub fn vertex_nbr(v: Vertex, dir: Dir) -> Vertex {
    match dir {
        Dir::N => v.up(),
        Dir::E => v.right(),
        Dir::S => v.down(),
        Dir::W => v.left(),
        Dir::NW => v.up().left(),
        Dir::NE => v.up().right(),
        Dir::SE => v.down().right(),
        Dir::SW => v.down().left(),
    }
}

// Helper functions for Color
pub fn color_is_player(color: Color) -> bool {
    use std::convert::TryFrom;
    Player::try_from(color).is_ok()
}

pub fn color_to_player(color: Color) -> Player {
    use std::convert::TryFrom;
    Player::try_from(color).expect("Color is not a player color")
}

pub fn color_to_showboard_char(color: Color) -> char {
    match color {
        Color::Black => '#',
        Color::White => 'O',
        Color::Empty => '.',
        Color::OffBoard => '$',
    }
}

// Type aliases for maps
pub type PlayerMap<T> = nat_map::NatMap<{ Player::COUNT }, Player, T>;
pub type VertexMap<T> = nat_map::NatMap<{ Vertex::COUNT }, Vertex, T>;
pub type ColorMap<T> = nat_map::NatMap<{ Color::COUNT }, Color, T>;
pub type MoveMap<T> = nat_map::NatMap<{ Move::COUNT }, Move, T>;
