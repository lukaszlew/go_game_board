use crate::fast_random::FastRandom;
use crate::types::{
    color_is_player, color_to_player, vertex_nbr, Color, ColorMap, Dir, Move, MoveMap, Nat, Player,
    PlayerMap, Vertex, VertexMap,
};

// Hash3x3Map uses Vec internally due to its massive size (2^20 elements)
pub struct Hash3x3Map<T> {
    data: Vec<T>,
}

impl<T: Default + Clone> Hash3x3Map<T> {
    pub fn new() -> Self {
        Self {
            data: vec![T::default(); Hash3x3::COUNT],
        }
    }
}

impl<T> std::ops::Index<Hash3x3> for Hash3x3Map<T> {
    type Output = T;

    fn index(&self, idx: Hash3x3) -> &Self::Output {
        &self.data[usize::from(idx) as usize]
    }
}

impl<T> std::ops::IndexMut<Hash3x3> for Hash3x3Map<T> {
    fn index_mut(&mut self, idx: Hash3x3) -> &mut Self::Output {
        &mut self.data[usize::from(idx) as usize]
    }
}

// Hash3x3 - perfect 20 bit hash (bitmask)
// bit mask from least significant
// N, E, S, W, NW, NE, SE, SW, aN, aE, aS, aW
// 2  2  2  2   2   2   2   2   1   1   1   1
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct Hash3x3(u32);

impl From<usize> for Hash3x3 {
    fn from(raw: usize) -> Self {
        Hash3x3(raw as u32)
    }
}

impl From<Hash3x3> for usize {
    fn from(hash: Hash3x3) -> usize {
        hash.0 as usize
    }
}

impl Nat for Hash3x3 {
    const COUNT: usize = 1 << 20; // 2^20
}

impl Hash3x3 {
    pub fn of_board(color_at: &VertexMap<Color>, v: Vertex) -> Self {
        // If the vertex itself is off-board, return empty hash
        if color_at[v] == Color::OffBoard {
            return Hash3x3::from(0);
        }
        let mut raw = 0u32;
        for dir in Dir::all() {
            raw |= (usize::from(color_at[vertex_nbr(v, dir)]) << (2 * usize::from(dir))) as u32;
        }
        Hash3x3(raw)
    }

    pub fn color_at(&self, dir: Dir) -> Color {
        Color::from((self.0 >> (2 * usize::from(dir))) as usize & 3)
    }

    pub fn set_color_at(&mut self, dir: Dir, color: Color) {
        self.0 &= !(3 << (2 * usize::from(dir)));
        self.0 |= (usize::from(color) << (2 * usize::from(dir))) as u32;
    }

    pub fn set_atari_bits(&mut self, b_n: bool, b_e: bool, b_s: bool, b_w: bool) {
        let mask = ((b_n as u32) << 16)
            | ((b_e as u32) << 17)
            | ((b_s as u32) << 18)
            | ((b_w as u32) << 19);
        self.0 |= mask;
    }

    pub fn unset_atari_bits(&mut self, b_n: bool, b_e: bool, b_s: bool, b_w: bool) {
        let mask = ((b_n as u32) << 16)
            | ((b_e as u32) << 17)
            | ((b_s as u32) << 18)
            | ((b_w as u32) << 19);
        self.0 &= !mask;
    }

    pub fn reset_atari_bits(&mut self) {
        self.0 &= (1 << 16) - 1;
    }

    pub fn is_in_atari(&self, dir: Dir) -> bool {
        debug_assert!(dir.is_simple4());
        (self.0 & (1 << (16 + usize::from(dir)))) != 0
    }

    pub fn is_legal(&self, pl: Player) -> bool {
        let mut color_cnt = ColorMap::<u32>::new();
        let mut atari_cnt = PlayerMap::<u32>::new();

        for dir in Dir::all() {
            if !dir.is_simple4() {
                continue;
            }
            let c = self.color_at(dir);
            color_cnt[c] += 1;
            if color_is_player(c) && self.is_in_atari(dir) {
                atari_cnt[color_to_player(c)] += 1;
            }
        }

        if color_cnt[Color::Empty] > 0 {
            return true;
        }
        if atari_cnt[pl.opponent()] > 0 {
            return true;
        }
        if atari_cnt[pl] < color_cnt[Color::from(pl)] {
            return true;
        }
        false
    }

    pub fn is_eyelike(&self, pl: Player) -> bool {
        let my_color = Color::from(pl);
        let enemy_color = Color::from(pl.opponent());

        // All 4 direct neighbors must be our color
        for i in 0..4 {
            let dir = Dir::from(i);
            let color = self.color_at(dir);
            if color != my_color && color != Color::OffBoard {
                return false;
            }
        }

        // Count enemy diagonal corners and off-board diagonal corners
        let mut enemy_diag_count = 0;
        let mut off_board_diag_count = 0;
        for i in 4..8 {
            let dir = Dir::from(i);
            let color = self.color_at(dir);
            if color == enemy_color {
                enemy_diag_count += 1;
            }
            if color == Color::OffBoard {
                off_board_diag_count += 1;
            }
        }

        // C++ logic: enemy_diag_count + (off_board_diag_count > 0 ? 1 : 0) < 2
        enemy_diag_count + if off_board_diag_count > 0 { 1 } else { 0 } < 2
    }
}

// Zobrist hash for the whole board position
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct Hash {
    hash: u64,
}

impl Hash {
    pub fn new() -> Self {
        Hash { hash: 0 }
    }

    pub fn set_zero(&mut self) {
        self.hash = 0;
    }

    pub fn randomize(&mut self, fr: &mut FastRandom) {
        // Match C++ initialization exactly
        self.hash = (fr.get_next_uint() as u64) << (0 * 16)
            ^ (fr.get_next_uint() as u64) << (1 * 16)
            ^ (fr.get_next_uint() as u64) << (2 * 16)
            ^ (fr.get_next_uint() as u64) << (3 * 16);
    }
}

impl std::ops::BitXorAssign for Hash {
    fn bitxor_assign(&mut self, other: Hash) {
        self.hash ^= other.hash;
    }
}

impl std::ops::BitXor for Hash {
    type Output = Hash;
    fn bitxor(self, other: Hash) -> Hash {
        Hash {
            hash: self.hash ^ other.hash,
        }
    }
}

// Zobrist table for position hashing
pub struct Zobrist {
    hashes: MoveMap<Hash>,
}

impl Zobrist {
    pub fn new() -> Self {
        let mut zobrist = Zobrist {
            hashes: MoveMap::new_with(Hash::new()),
        };

        // Initialize exactly like C++ with seed 123
        let mut rng = FastRandom::new(123);

        // Match C++ iteration order: ForEachNat(Player, pl) { ForEachNat(Vertex, v) { ... } }
        for pl_raw in 0..2 {
            let pl = Player::from(pl_raw);
            for v_raw in 0..Vertex::COUNT as usize {
                let v = Vertex::from(v_raw);
                let mv = Move::of_player_vertex(pl, v);
                zobrist.hashes[mv].randomize(&mut rng);
            }
        }

        zobrist
    }

    pub fn of_player_vertex(&self, pl: Player, v: Vertex) -> Hash {
        self.hashes[Move::of_player_vertex(pl, v)]
    }
}

// Global Zobrist instance
lazy_static::lazy_static! {
    pub static ref ZOBRIST: Zobrist = Zobrist::new();
}
