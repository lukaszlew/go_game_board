use crate::hash::{Hash, Hash3x3, ZOBRIST};
use crate::nat_set::NatSet;
use crate::types::{
    color_is_player, color_to_player, color_to_showboard_char, vertex_nbr, vertex_of_coords_full,
    Color, Dir, Nat, Player, PlayerMap, Vertex, VertexMap, MAX_BOARD_SIZE,
};
use arrayvec::ArrayVec;

const K_AREA: usize = MAX_BOARD_SIZE * MAX_BOARD_SIZE;

// Neighbor counter using bitfield like C++
#[derive(Copy, Clone, Debug)]
pub struct NbrCounter {
    bitfield: u32,
}

impl Default for NbrCounter {
    fn default() -> Self {
        Self::empty()
    }
}

impl NbrCounter {
    const MAX: u32 = 4;
    #[allow(dead_code)]
    const F_SIZE: u32 = 4;
    const F_SHIFT: [u32; 3] = [0, 4, 8];

    pub fn empty() -> Self {
        Self::of_counts(0, 0, Self::MAX)
    }

    pub fn of_counts(black_cnt: u32, white_cnt: u32, empty_cnt: u32) -> Self {
        assert!(black_cnt <= Self::MAX);
        assert!(white_cnt <= Self::MAX);
        assert!(empty_cnt <= Self::MAX);
        NbrCounter {
            bitfield: (black_cnt << Self::F_SHIFT[0])
                + (white_cnt << Self::F_SHIFT[1])
                + (empty_cnt << Self::F_SHIFT[2]),
        }
    }

    pub fn player_inc(&mut self, player: Player) {
        // When a player stone is added, we increment that player's count and decrement empty count
        let player_inc_tab = [
            ((1u32 << Self::F_SHIFT[0]) as i32 - (1u32 << Self::F_SHIFT[2]) as i32) as u32,
            ((1u32 << Self::F_SHIFT[1]) as i32 - (1u32 << Self::F_SHIFT[2]) as i32) as u32,
        ];
        self.bitfield = self
            .bitfield
            .wrapping_add(player_inc_tab[usize::from(player)]);
    }

    pub fn player_dec(&mut self, player: Player) {
        // When a player stone is removed, we decrement that player's count and increment empty count
        let player_inc_tab = [
            ((1u32 << Self::F_SHIFT[0]) as i32 - (1u32 << Self::F_SHIFT[2]) as i32) as u32,
            ((1u32 << Self::F_SHIFT[1]) as i32 - (1u32 << Self::F_SHIFT[2]) as i32) as u32,
        ];
        self.bitfield = self
            .bitfield
            .wrapping_sub(player_inc_tab[usize::from(player) as usize]);
    }

    pub fn off_board_inc(&mut self) {
        let off_board_inc_val = (1u32 << Self::F_SHIFT[0])
            .wrapping_add(1u32 << Self::F_SHIFT[1])
            .wrapping_sub(1u32 << Self::F_SHIFT[2]);
        self.bitfield = self.bitfield.wrapping_add(off_board_inc_val);
    }

    #[allow(dead_code)]
    pub fn empty_cnt(&self) -> u32 {
        self.bitfield >> Self::F_SHIFT[2]
    }

    #[allow(dead_code)]
    pub fn player_cnt(&self, pl: Player) -> u32 {
        let f_mask = (1 << Self::F_SIZE) - 1;
        (self.bitfield >> Self::F_SHIFT[usize::from(pl) as usize]) & f_mask
    }

    pub fn player_cnt_is_max(&self, pl: Player) -> bool {
        let player_cnt_is_max_mask = [Self::MAX << Self::F_SHIFT[0], Self::MAX << Self::F_SHIFT[1]];
        (player_cnt_is_max_mask[usize::from(pl) as usize] & self.bitfield)
            == player_cnt_is_max_mask[usize::from(pl) as usize]
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Chain {
    pub lib_cnt: u32,
    pub lib_sum: u32,
    pub lib_sum2: u32,
    pub size: u32,
    pub atari_v: Vertex,
}

impl Default for Chain {
    fn default() -> Self {
        Chain {
            lib_cnt: 0,
            lib_sum: 0,
            lib_sum2: 0,
            size: 0,
            atari_v: Vertex::none(),
        }
    }
}

impl Chain {
    pub fn reset(&mut self) {
        self.lib_cnt = 0;
        self.lib_sum = 0;
        self.lib_sum2 = 0;
        self.size = 0;
        self.atari_v = Vertex::none();
    }

    pub fn reset_off_board(&mut self) {
        self.lib_cnt = 2; // This is needed to not try to remove offboard guards
        self.lib_sum = 1;
        self.lib_sum2 = 1;
        self.size = 100;
        self.atari_v = Vertex::none();
    }

    pub fn add_lib(&mut self, v: Vertex) {
        self.lib_cnt = self.lib_cnt.wrapping_add(1);
        self.lib_sum = self.lib_sum.wrapping_add(usize::from(v) as u32);
        self.lib_sum2 = self
            .lib_sum2
            .wrapping_add((usize::from(v) * usize::from(v)) as u32);
    }

    pub fn sub_lib(&mut self, v: Vertex) {
        self.lib_cnt = self.lib_cnt.wrapping_sub(1);
        self.lib_sum = self.lib_sum.wrapping_sub(usize::from(v) as u32);
        self.lib_sum2 = self
            .lib_sum2
            .wrapping_sub(usize::from(v) as u32 * usize::from(v) as u32);
    }

    pub fn merge(&mut self, other: &Chain) {
        self.lib_cnt += other.lib_cnt;
        self.lib_sum += other.lib_sum;
        self.lib_sum2 += other.lib_sum2;
        self.size += other.size;
    }

    pub fn is_captured(&self) -> bool {
        self.lib_cnt == 0
    }

    pub fn is_in_atari(&self) -> bool {
        self.lib_cnt * self.lib_sum2 == self.lib_sum * self.lib_sum
    }

    #[allow(dead_code)]
    pub fn atari_vertex(&self) -> Vertex {
        assert!(self.is_in_atari());
        self.atari_v
    }
}

pub struct Board {
    move_no: usize,
    komi: f32,
    pub color_at: VertexMap<Color>,
    ko_v: Vertex,
    last_player: Player,
    last_play: PlayerMap<Vertex>,
    board_width: usize,
    board_height: usize,

    // Positional hash
    hash: Hash,

    player_v_cnt: PlayerMap<u32>,
    chain_next_v: VertexMap<Vertex>,
    chain_id: VertexMap<Vertex>,
    chain: VertexMap<Chain>,

    nbr_cnt: VertexMap<NbrCounter>,

    empty_v_cnt: u32,
    empty_v: [Vertex; K_AREA],
    empty_pos: VertexMap<u32>,

    play_count: VertexMap<u32>,

    hash3x3: VertexMap<Hash3x3>,
    hash3x3_changed: ArrayVec<Vertex, K_AREA>,
    tmp_vertex_set: NatSet<{ Vertex::COUNT }, Vertex>,
}

impl Board {
    pub fn new() -> Self {
        Self::with_size(9, 9)
    }

    pub fn with_size(width: usize, height: usize) -> Self {
        assert!(
            width > 0 && width <= MAX_BOARD_SIZE,
            "Board width must be between 1 and {}",
            MAX_BOARD_SIZE
        );
        assert!(
            height > 0 && height <= MAX_BOARD_SIZE,
            "Board height must be between 1 and {}",
            MAX_BOARD_SIZE
        );

        let mut board = Board {
            move_no: 0,
            komi: 6.5,
            color_at: VertexMap::new_with(Color::Empty),
            ko_v: Vertex::none(),
            last_player: Player::White,
            last_play: PlayerMap::new_with(Vertex::none()),
            board_width: width,
            board_height: height,
            hash: Hash::new(),

            player_v_cnt: PlayerMap::new(),
            chain_next_v: VertexMap::new_with(Vertex::none()),
            chain_id: VertexMap::new_with(Vertex::none()),
            chain: VertexMap::new(),

            nbr_cnt: VertexMap::new(),

            empty_v_cnt: 0,
            empty_v: [Vertex::none(); K_AREA],
            empty_pos: VertexMap::new(),

            play_count: VertexMap::new(),

            hash3x3: VertexMap::new(),
            hash3x3_changed: ArrayVec::new(),
            tmp_vertex_set: NatSet::<{ Vertex::COUNT }, Vertex>::new(),
        };

        board.clear();
        board
    }

    pub fn clear(&mut self) {
        self.move_no = 0;
        self.last_player = Player::White;
        self.ko_v = Vertex::none();

        // Initialize all vertices
        for v in Vertex::all() {
            self.color_at[v] = Color::OffBoard;
            self.chain_next_v[v] = v;
            self.chain_id[v] = v;
            self.nbr_cnt[v] = NbrCounter::empty();
            self.play_count[v] = 0;
            self.empty_pos[v] = 0;
            self.chain[v].reset_off_board();
        }

        // Clear empty vertex list
        self.empty_v_cnt = 0;

        // Set up board positions - only within the actual board size
        for v in Vertex::all() {
            if self.is_within_board(v) {
                self.color_at[v] = Color::Empty;
                self.chain[v].reset();

                // Add to empty list
                self.empty_pos[v] = self.empty_v_cnt;
                self.empty_v[self.empty_v_cnt as usize] = v;
                self.empty_v_cnt += 1;
            }
        }

        // Update neighbor counts for edges
        for v in Vertex::all() {
            if self.color_at[v] == Color::Empty {
                self.nbr_cnt[v] = NbrCounter::empty();

                // Count off-board neighbors
                for_each_4_nbr!(v, nbr_v, {
                    if self.color_at[nbr_v] == Color::OffBoard {
                        self.nbr_cnt[v].off_board_inc();
                    }
                });
            }
        }

        self.player_v_cnt[Player::Black] = 0;
        self.player_v_cnt[Player::White] = 0;

        self.last_play[Player::Black] = Vertex::none();
        self.last_play[Player::White] = Vertex::none();

        // Initialize hash3x3 for all vertices
        for v in Vertex::all() {
            self.hash3x3[v] = Hash3x3::of_board(&self.color_at, v);
        }
        self.hash3x3_changed.clear();

        // Recalculate positional hash
        self.hash = self.recalc_hash();
    }

    fn is_within_board(&self, v: Vertex) -> bool {
        let row = v.row() as i32 + 1;
        let col = v.column() as i32 + 1;
        row > 0 && row <= self.board_height as i32 && col > 0 && col <= self.board_width as i32
    }

    pub fn act_player(&self) -> Player {
        self.last_player.opponent()
    }

    pub fn color_at(&self, v: Vertex) -> Color {
        self.color_at[v]
    }

    pub fn empty_vertex_count(&self) -> usize {
        self.empty_v_cnt as usize
    }

    #[allow(dead_code)]
    pub fn move_no(&self) -> usize {
        self.move_no
    }

    pub fn empty_vertex(&self, idx: usize) -> Vertex {
        self.empty_v[idx]
    }

    #[allow(dead_code)]
    pub fn is_legal(&self, player: Player, v: Vertex) -> bool {
        if v == Vertex::pass() {
            return true;
        }

        if self.color_at[v] != Color::Empty || v == self.ko_v {
            return false;
        }

        // Check for suicide - match C++ exactly
        if self.nbr_cnt[v].empty_cnt() > 0 {
            return true;
        }

        // Match C++ logic exactly - decrement once per NEIGHBOR, not per chain
        let mut not_suicide = false;

        // C++ decrements each neighbor's chain, even if same chain appears multiple times
        let mut temp_libs = [0i32; 625]; // Use i32 to handle multiple decrements

        // Initialize with original liberties
        for_each_4_nbr!(v, nbr_v, {
            let chain_id = self.chain_id[nbr_v];
            if temp_libs[usize::from(chain_id) as usize] == 0 {
                temp_libs[usize::from(chain_id) as usize] = self.chain[chain_id].lib_cnt as i32;
            }
        });

        // Decrement once per neighbor (C++ behavior)
        for_each_4_nbr!(v, nbr_v, {
            let chain_id = self.chain_id[nbr_v];
            temp_libs[usize::from(chain_id) as usize] -= 1;
        });

        // Check each neighbor
        for_each_4_nbr!(v, nbr_v, {
            if color_is_player(self.color_at[nbr_v]) {
                let chain_id = self.chain_id[nbr_v];
                let atari = temp_libs[usize::from(chain_id) as usize] == 0;
                let is_same_color = color_to_player(self.color_at[nbr_v]) == player;

                // C++ logic: atari != (color_at[nbr_v].ToPlayer() == player)
                not_suicide |= atari != is_same_color;
            }
        });

        not_suicide
    }

    pub fn play_legal(&mut self, player: Player, v: Vertex) {
        // Clear tracking state
        self.tmp_vertex_set.clear();
        self.hash3x3_changed.clear();

        self.last_play[player] = v;
        self.last_player = player;
        self.move_no += 1;

        if v == Vertex::pass() {
            self.ko_v = Vertex::none();
            return;
        }

        self.play_count[v] += 1;
        self.place_stone(player, v);

        // Now handle neighbors similar to C++ update_neighbour
        let color = Color::from(player);
        let mut captured_cnt = 0;
        let mut last_captured_v = Vertex::none();

        for_each_4_nbr!(v, nbr_v, {
            let nbr_color = self.color_at[nbr_v];
            if color_is_player(nbr_color) {
                if nbr_color != color {
                    // Enemy chain
                    let nbr_chain_id = self.chain_id[nbr_v];
                    if self.chain[nbr_chain_id].is_captured() {
                        captured_cnt += self.chain[nbr_chain_id].size;
                        last_captured_v = nbr_v;
                        self.remove_chain(nbr_v);
                    } else {
                        // Reduced liberty of opponent - check for atari
                        self.maybe_in_atari(nbr_v);
                    }
                } else {
                    // Same color - merge chains if needed
                    let nbr_chain_id = self.chain_id[nbr_v];
                    if self.chain_id[v] != nbr_chain_id {
                        if self.chain[self.chain_id[v]].size > self.chain[nbr_chain_id].size {
                            self.merge_chains(v, nbr_v);
                        } else {
                            self.merge_chains(nbr_v, v);
                        }
                    }
                }
            }
        });

        // Update ko
        if captured_cnt == 1
            && self.chain[self.chain_id[v]].size == 1
            && self.chain[self.chain_id[v]].lib_cnt == 1
        {
            self.ko_v = last_captured_v;
        } else {
            self.ko_v = Vertex::none();
        }

        // Check for atari of the played chain
        self.maybe_in_atari(v);
    }

    fn place_stone(&mut self, player: Player, v: Vertex) {
        assert!(
            self.color_at[v] == Color::Empty,
            "Trying to place {:?} stone at {}-{} which has color {}",
            player,
            v.row() as i32 + 1,
            v.column() as i32 + 1,
            color_to_showboard_char(self.color_at[v])
        );

        // Remove from empty list - match C++ exactly
        self.empty_v_cnt -= 1;
        self.empty_pos[self.empty_v[self.empty_v_cnt as usize]] = self.empty_pos[v];
        self.empty_v[self.empty_pos[v] as usize] = self.empty_v[self.empty_v_cnt as usize];

        // Place stone
        let color = Color::from(player);
        self.color_at[v] = color;
        self.player_v_cnt[player] += 1;

        // Update positional hash
        self.hash ^= ZOBRIST.of_player_vertex(player, v);

        // Update hash3x3 for all neighbors
        for dir in Dir::all() {
            let nbr = vertex_nbr(v, dir);
            self.hash3x3[nbr].set_color_at(dir.opposite(), color);
            if !self.tmp_vertex_set.is_marked(nbr) && self.color_at[nbr] == Color::Empty {
                self.hash3x3_changed.push(nbr);
                self.tmp_vertex_set.mark(nbr);
            }
        }

        // Initialize chain
        self.chain_id[v] = v;
        self.chain_next_v[v] = v;
        self.chain[v].reset();
        self.chain[v].size = 1;

        // Process all neighbors in one loop like C++
        for_each_4_nbr!(v, nbr_v, {
            let nbr_color = self.color_at[nbr_v];

            // Update neighbor counts - ALL neighbors lose an empty neighbor
            self.nbr_cnt[nbr_v].player_inc(player);

            if nbr_color == Color::Empty {
                // Add liberty for the new stone
                self.chain[v].add_lib(nbr_v);
            } else {
                // Subtract liberty from neighbor chains (both player and off-board)
                if color_is_player(nbr_color) {
                    let nbr_chain_id = self.chain_id[nbr_v];
                    self.chain[nbr_chain_id].sub_lib(v);
                } else if nbr_color == Color::OffBoard {
                    // For off-board, C++ uses chain_at which accesses chain[nbr_v]
                    self.chain[nbr_v].sub_lib(v);
                }
            }
        });
    }

    fn merge_chains(&mut self, v_base: Vertex, v_add: Vertex) {
        let base_id = self.chain_id[v_base];
        let add_id = self.chain_id[v_add];

        if base_id == add_id {
            return;
        }

        // Merge chain data - copy to avoid borrow issue
        let add_chain = self.chain[add_id].clone();
        self.chain[base_id].merge(&add_chain);

        // Update chain IDs
        let mut current = v_add;
        loop {
            self.chain_id[current] = base_id;
            current = self.chain_next_v[current];
            if current == v_add {
                break;
            }
        }

        // Merge linked lists
        let base_next = self.chain_next_v[v_base];
        let add_next = self.chain_next_v[v_add];
        self.chain_next_v[v_base] = add_next;
        self.chain_next_v[v_add] = base_next;
    }

    fn maybe_in_atari(&mut self, v: Vertex) {
        // Update atari bits in hash3x3
        if self.color_at[v] == Color::Empty || self.color_at[v] == Color::OffBoard {
            return;
        }
        let chain_id = self.chain_id[v];
        if !self.chain[chain_id].is_in_atari() {
            return;
        }

        // Calculate atari vertex from lib_sum / lib_cnt (like C++)
        let chain = &self.chain[chain_id];
        assert!(
            chain.lib_sum % chain.lib_cnt == 0,
            "lib_sum % lib_cnt should be 0"
        );
        let av = Vertex::from((chain.lib_sum / chain.lib_cnt) as usize);
        if self.color_at[av] != Color::Empty {
            return; // Safety check
        }

        self.chain[chain_id].atari_v = av;

        // Set atari bits based on which neighbors belong to the same chain
        self.hash3x3[av].set_atari_bits(
            self.chain_id[vertex_nbr(av, Dir::N)] == chain_id,
            self.chain_id[vertex_nbr(av, Dir::E)] == chain_id,
            self.chain_id[vertex_nbr(av, Dir::S)] == chain_id,
            self.chain_id[vertex_nbr(av, Dir::W)] == chain_id,
        );

        if !self.tmp_vertex_set.is_marked(av) {
            self.hash3x3_changed.push(av);
            self.tmp_vertex_set.mark(av);
        }
    }

    fn maybe_in_atari_end(&mut self, v: Vertex) {
        // Update atari bits in hash3x3
        if !color_is_player(self.color_at[v]) {
            return;
        }
        let chain_id = self.chain_id[v];
        if self.chain[chain_id].is_captured() {
            return;
        }
        if !self.chain[chain_id].is_in_atari() {
            return;
        }

        // Calculate atari vertex from lib_sum / lib_cnt (like C++)
        let chain = &self.chain[chain_id];
        assert!(
            chain.lib_sum % chain.lib_cnt == 0,
            "lib_sum % lib_cnt should be 0"
        );
        let av = Vertex::from((chain.lib_sum / chain.lib_cnt) as usize);
        if self.color_at[av] != Color::Empty {
            return; // Safety check
        }

        self.chain[chain_id].atari_v = Vertex::none();

        // Unset atari bits
        self.hash3x3[av].unset_atari_bits(
            self.chain_id[vertex_nbr(av, Dir::N)] == chain_id,
            self.chain_id[vertex_nbr(av, Dir::E)] == chain_id,
            self.chain_id[vertex_nbr(av, Dir::S)] == chain_id,
            self.chain_id[vertex_nbr(av, Dir::W)] == chain_id,
        );

        if !self.tmp_vertex_set.is_marked(av) {
            self.hash3x3_changed.push(av);
            self.tmp_vertex_set.mark(av);
        }
    }

    fn remove_chain(&mut self, v: Vertex) {
        let color = self.color_at[v];
        assert!(color_is_player(color));
        let player = color_to_player(color);

        // First pass: remove all stones
        let mut current = v;
        loop {
            let act_v = current;

            // Add to empty list
            self.empty_pos[act_v] = self.empty_v_cnt;
            self.empty_v[self.empty_v_cnt as usize] = act_v;
            self.empty_v_cnt += 1;

            // Remove stone
            self.color_at[act_v] = Color::Empty;
            self.chain_id[act_v] = act_v;
            self.player_v_cnt[player] -= 1;

            // Update positional hash
            self.hash ^= ZOBRIST.of_player_vertex(player, act_v);

            // Update hash3x3 for removed stone
            self.hash3x3[act_v].reset_atari_bits();
            if !self.tmp_vertex_set.is_marked(act_v) {
                self.hash3x3_changed.push(act_v);
                self.tmp_vertex_set.mark(act_v);
            }

            // Update hash3x3 for all neighbors
            for dir in Dir::all() {
                let nbr = vertex_nbr(act_v, dir);
                self.hash3x3[nbr].set_color_at(dir.opposite(), Color::Empty);
                if !self.tmp_vertex_set.is_marked(nbr) && self.color_at[nbr] == Color::Empty {
                    self.hash3x3_changed.push(nbr);
                    self.tmp_vertex_set.mark(nbr);
                }
            }

            // Update neighbor counts
            for_each_4_nbr!(act_v, nbr_v, {
                self.nbr_cnt[nbr_v].player_dec(player);
            });

            current = self.chain_next_v[current];
            if current == v {
                break;
            }
        }

        // Second pass: update liberties and reset chain_next_v
        current = v;
        loop {
            let act_v = current;

            // Update liberties for neighboring chains
            for_each_4_nbr!(act_v, nbr_v, {
                let _nbr_color = self.color_at[nbr_v];
                // Must call maybe_in_atari_end BEFORE adding liberty (like C++)
                self.maybe_in_atari_end(nbr_v);
                self.chain[self.chain_id[nbr_v]].add_lib(act_v);
            });

            let next = self.chain_next_v[current];
            self.chain_next_v[current] = current;
            current = next;

            if current == v {
                break;
            }
        }
    }

    #[allow(dead_code)]
    pub fn print_all_maps(&self) {
        // Print color_at
        println!("color_at:");
        let mut str_map = VertexMap::<String>::new();
        for v in Vertex::all() {
            str_map[v] = color_to_showboard_char(self.color_at[v]).to_string();
        }
        println!("{}", vmap_to_ascii_art_with_sentinels(&str_map));

        // Print chain_id
        println!("chain_id:");
        for v in Vertex::all() {
            str_map[v] = format!("{}", usize::from(self.chain_id[v]) % 100);
        }
        println!("{}", vmap_to_ascii_art_with_sentinels(&str_map));

        // Print chain_next_v
        println!("chain_next_v:");
        for v in Vertex::all() {
            str_map[v] = format!("{}", usize::from(self.chain_next_v[v]) % 100);
        }
        println!("{}", vmap_to_ascii_art_with_sentinels(&str_map));

        // Print nbr_cnt.empty_cnt()
        println!("nbr_cnt.empty_cnt():");
        for v in Vertex::all() {
            str_map[v] = format!("{}", self.nbr_cnt[v].empty_cnt());
        }
        println!("{}", vmap_to_ascii_art_with_sentinels(&str_map));

        // Print hash3x3
        println!("hash3x3:");
        for v in Vertex::all() {
            str_map[v] = format!("{}", usize::from(self.hash3x3[v]));
        }
        println!("{}", vmap_to_ascii_art_with_sentinels(&str_map));

        // Print empty_pos
        println!("empty_pos:");
        for v in Vertex::all() {
            if self.color_at[v] == Color::Empty && self.is_within_board(v) {
                str_map[v] = format!("{}", self.empty_pos[v]);
            } else {
                str_map[v] = "-".to_string();
            }
        }
        println!("{}", vmap_to_ascii_art_with_sentinels(&str_map));

        // Print play_count
        println!("play_count:");
        for v in Vertex::all() {
            str_map[v] = format!("{}", self.play_count[v]);
        }
        println!("{}", vmap_to_ascii_art_with_sentinels(&str_map));

        // Print chain.lib_cnt
        println!("chain.lib_cnt:");
        for v in Vertex::all() {
            if color_is_player(self.color_at[v]) {
                str_map[v] = format!("{}", self.chain[self.chain_id[v]].lib_cnt);
            } else {
                str_map[v] = "-".to_string();
            }
        }
        println!("{}", vmap_to_ascii_art_with_sentinels(&str_map));

        // Print chain.size
        println!("chain.size:");
        for v in Vertex::all() {
            if color_is_player(self.color_at[v]) {
                str_map[v] = format!("{}", self.chain[self.chain_id[v]].size);
            } else {
                str_map[v] = "-".to_string();
            }
        }
        println!("{}", vmap_to_ascii_art_with_sentinels(&str_map));
    }

    pub fn hash3x3_at(&self, v: Vertex) -> Hash3x3 {
        self.hash3x3[v]
    }

    pub fn hash3x3_changed_count(&self) -> usize {
        self.hash3x3_changed.len()
    }

    pub fn hash3x3_changed(&self, ii: usize) -> Vertex {
        self.hash3x3_changed[ii]
    }

    pub fn ko_vertex(&self) -> Vertex {
        self.ko_v
    }

    #[allow(dead_code)]
    pub fn positional_hash(&self) -> Hash {
        self.hash
    }

    fn recalc_hash(&self) -> Hash {
        let mut new_hash = Hash::new();
        new_hash.set_zero();

        for v in Vertex::all() {
            if color_is_player(self.color_at[v]) {
                new_hash ^= ZOBRIST.of_player_vertex(color_to_player(self.color_at[v]), v);
            }
        }

        new_hash
    }

    pub fn last_player(&self) -> Player {
        self.last_player
    }

    pub fn last_vertex(&self) -> Vertex {
        if self.move_no == 0 {
            Vertex::none()
        } else {
            self.last_play[self.last_player]
        }
    }

    pub fn both_player_pass(&self) -> bool {
        self.last_play[Player::Black] == Vertex::pass()
            && self.last_play[Player::White] == Vertex::pass()
    }

    pub fn playout_winner(&self) -> Player {
        let score = self.playout_score();
        // In C++: Player::OfRaw(score <= 0)
        // Returns White (1) if score <= 0, Black (0) if score > 0
        if score <= 0 {
            Player::White
        } else {
            Player::Black
        }
    }

    pub fn playout_score(&self) -> i32 {
        let stone_score = self.stone_score();
        let eye_score = self.calculate_eye_score();
        stone_score + eye_score
    }

    fn stone_score(&self) -> i32 {
        // komi_inverse + black_stones - white_stones
        // In C++, komi_inverse = ceil(-komi)
        let komi_inverse = (-(self.komi)).ceil() as i32;
        komi_inverse + self.player_v_cnt[Player::Black] as i32
            - self.player_v_cnt[Player::White] as i32
    }

    fn calculate_eye_score(&self) -> i32 {
        let mut eye_score = 0;

        for i in 0..self.empty_v_cnt {
            let v = self.empty_v[i as usize];
            eye_score += self.eye_score(v);
        }

        eye_score
    }

    fn eye_score(&self, v: Vertex) -> i32 {
        // Returns 1 if all neighbors are black (black eye), -1 if all white (white eye), 0 otherwise
        let black_eye = self.nbr_cnt[v].player_cnt_is_max(Player::Black);
        let white_eye = self.nbr_cnt[v].player_cnt_is_max(Player::White);

        (black_eye as i32) - (white_eye as i32)
    }

    pub fn move_count(&self) -> usize {
        self.move_no
    }

    pub fn load(&mut self, source: &Board) {
        *self = source.clone();
    }

    #[allow(dead_code)]
    pub fn tromp_taylor_score(&self) -> f32 {
        let mut score = self.komi;

        for v in Vertex::all() {
            if !self.is_within_board(v) {
                continue;
            }

            let color = self.color_at[v];
            if color == Color::Black {
                score += 1.0;
            } else if color == Color::White {
                score -= 1.0;
            } else if color == Color::Empty {
                // Check if it's surrounded by only one color
                let mut black_neighbors = false;
                let mut white_neighbors = false;

                for_each_4_nbr!(v, nbr_v, {
                    let nbr_color = self.color_at[nbr_v];
                    if nbr_color == Color::Black {
                        black_neighbors = true;
                    } else if nbr_color == Color::White {
                        white_neighbors = true;
                    }
                });

                if black_neighbors && !white_neighbors {
                    score += 1.0;
                } else if white_neighbors && !black_neighbors {
                    score -= 1.0;
                }
            }
        }

        score
    }
}

impl Clone for Board {
    fn clone(&self) -> Self {
        Board {
            move_no: self.move_no,
            komi: self.komi,
            color_at: self.color_at.clone(),
            ko_v: self.ko_v,
            last_player: self.last_player,
            last_play: self.last_play.clone(),
            board_width: self.board_width,
            board_height: self.board_height,
            hash: self.hash,
            player_v_cnt: self.player_v_cnt.clone(),
            chain_next_v: self.chain_next_v.clone(),
            chain_id: self.chain_id.clone(),
            chain: self.chain.clone(),
            nbr_cnt: self.nbr_cnt.clone(),
            empty_v_cnt: self.empty_v_cnt,
            empty_v: self.empty_v.clone(),
            empty_pos: self.empty_pos.clone(),
            play_count: self.play_count.clone(),
            hash3x3: self.hash3x3.clone(),
            hash3x3_changed: self.hash3x3_changed.clone(),
            tmp_vertex_set: NatSet::<{ Vertex::COUNT }, Vertex>::new(), // Don't need to clone this
        }
    }
}

// Macro for iterating over 4 neighbors
macro_rules! for_each_4_nbr {
    ($center_v:expr, $nbr_v:ident, $block:block) => {
        {
            let $nbr_v = $center_v.up(); $block
            let $nbr_v = $center_v.left(); $block
            let $nbr_v = $center_v.right(); $block
            let $nbr_v = $center_v.down(); $block
        }
    };
}

use for_each_4_nbr;

#[allow(dead_code)]
pub fn vmap_to_ascii_art_with_sentinels(str_map: &VertexMap<String>) -> String {
    let mut result = String::new();

    for row in 0..MAX_BOARD_SIZE + 2 {
        for col in 0..MAX_BOARD_SIZE + 2 {
            let v = vertex_of_coords_full(row as i32, col as i32);
            result.push_str(&str_map[v]);
            result.push(' ');
        }
        result.push('\n');
    }

    result
}
