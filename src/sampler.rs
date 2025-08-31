use crate::board::Board;
use crate::fast_random::FastRandom;
use crate::gammas::{Gammas, GAMMAS_ACCURACY};
use crate::nat_set::NatSet;
use crate::types::{vertex_nbr, Color, Dir, Nat, Player, PlayerMap, Vertex, VertexMap};

pub struct Sampler {
    act_gamma: VertexMap<PlayerMap<f64>>,
    act_gamma_sum: PlayerMap<f64>,
    proximity_bonus: [f64; 2],

    is_in_local: NatSet<{ Vertex::COUNT }, Vertex>,
    local_vertices: Vec<Vertex>,
    local_gamma: VertexMap<f64>,
    total_non_local_gamma: f64,
    total_local_gamma: f64,

    ko_v: Vertex,
}

impl Sampler {
    pub fn new(_board: &Board, _gammas: &Gammas) -> Self {
        let mut sampler = Sampler {
            act_gamma: VertexMap::new(),
            act_gamma_sum: PlayerMap::new(),
            proximity_bonus: [10.0, 10.0],

            is_in_local: NatSet::<{ Vertex::COUNT }, Vertex>::new(),
            local_vertices: Vec::with_capacity(100),
            local_gamma: VertexMap::new(),
            total_non_local_gamma: 0.0,
            total_local_gamma: 0.0,

            ko_v: Vertex::none(),
        };

        // Initialize act_gamma
        for pl in Player::all() {
            for v in Vertex::all() {
                sampler.act_gamma[v][pl] = 0.0;
            }
            sampler.act_gamma_sum[pl] = 0.0;
        }

        sampler
    }

    pub fn new_playout(&mut self, board: &Board, gammas: &Gammas) {
        // Prepare act_gamma and act_gamma_sum
        for pl in Player::all() {
            self.act_gamma_sum[pl] = 0.0;
            for v in Vertex::all() {
                self.act_gamma[v][pl] = 0.0;
            }

            for ii in 0..board.empty_vertex_count() {
                let v = board.empty_vertex(ii);
                self.act_gamma[v][pl] = gammas.get(board.hash3x3_at(v), pl);
                self.act_gamma_sum[pl] += self.act_gamma[v][pl];
            }
        }

        let act_pl = board.act_player();
        self.ko_v = board.ko_vertex();
        if self.ko_v != Vertex::none() {
            self.act_gamma_sum[act_pl] -= self.act_gamma[self.ko_v][act_pl];
            self.act_gamma[self.ko_v][act_pl] = 0.0;
        }
    }

    pub fn move_played(&mut self, board: &Board, gammas: &Gammas) {
        let last_pl = board.last_player();
        let last_v = board.last_vertex();

        // Restore gamma after ko_ban lifted
        let _old_gamma = self.act_gamma[self.ko_v][last_pl];
        let hash = board.hash3x3_at(self.ko_v);
        let new_gamma = gammas.get(hash, last_pl);
        self.act_gamma[self.ko_v][last_pl] = new_gamma;
        self.act_gamma_sum[last_pl] += new_gamma;

        for pl in Player::all() {
            // One new occupied intersection
            let _old_val = self.act_gamma[last_v][pl];
            self.act_gamma_sum[pl] -= self.act_gamma[last_v][pl];
            self.act_gamma[last_v][pl] = 0.0;

            // All new gammas
            let n = board.hash3x3_changed_count();
            for ii in 0..n {
                let v = board.hash3x3_changed(ii);

                self.act_gamma_sum[pl] -= self.act_gamma[v][pl];
                self.act_gamma[v][pl] = gammas.get(board.hash3x3_at(v), pl);
                self.act_gamma_sum[pl] += self.act_gamma[v][pl];
            }
        }

        // New illegal ko point
        let act_pl = board.act_player();
        self.ko_v = board.ko_vertex();

        self.act_gamma_sum[act_pl] -= self.act_gamma[self.ko_v][act_pl];
        self.act_gamma[self.ko_v][act_pl] = 0.0;
    }

    pub fn sample_move(&mut self, board: &Board, random: &mut FastRandom) -> Vertex {
        let pl = board.act_player();

        if self.act_gamma_sum[pl] < GAMMAS_ACCURACY {
            return Vertex::pass();
        }

        self.calculate_local_gammas(board);

        // Draw sample
        let total_gamma = self.total_non_local_gamma + self.total_local_gamma;
        let sample = random.next_double(total_gamma);

        // Local move?
        if sample < self.total_local_gamma {
            self.sample_local_move(sample)
        } else {
            let sample = sample - self.total_local_gamma;
            self.sample_non_local_move(board, sample)
        }
    }

    fn calculate_local_gammas(&mut self, board: &Board) {
        let pl = board.act_player();

        self.is_in_local.clear();
        self.local_vertices.clear();
        self.total_non_local_gamma = self.act_gamma_sum[pl];
        self.total_local_gamma = 0.0;

        let last_v = board.last_vertex();

        if board.color_at(last_v) != Color::OffBoard {
            for d in Dir::all() {
                let nbr = vertex_nbr(last_v, d);
                self.ensure_local(nbr, pl);
                self.local_gamma[nbr] *= self.proximity_bonus[d.proximity()];
            }
        }

        for ii in 0..self.local_vertices.len() {
            let local_v = self.local_vertices[ii];
            self.total_local_gamma += self.local_gamma[local_v];
        }
    }

    fn ensure_local(&mut self, v: Vertex, pl: Player) {
        if !self.is_in_local.is_marked(v) {
            self.is_in_local.mark(v);
            self.local_vertices.push(v);
            self.local_gamma[v] = self.act_gamma[v][pl];
            self.total_non_local_gamma -= self.act_gamma[v][pl];
        }
    }

    fn sample_local_move(&self, sample: f64) -> Vertex {
        let mut local_gamma_sum = 0.0;
        for ii in 0..self.local_vertices.len() {
            let nbr = self.local_vertices[ii];
            local_gamma_sum += self.local_gamma[nbr];
            if local_gamma_sum >= sample {
                return nbr;
            }
        }
        panic!("Should not reach here");
    }

    fn sample_non_local_move(&self, board: &Board, sample: f64) -> Vertex {
        let pl = board.act_player();
        let mut sum = 0.0;

        for ii in 0..board.empty_vertex_count() {
            let v = board.empty_vertex(ii);
            if self.is_in_local.is_marked(v) {
                continue;
            }
            sum += self.act_gamma[v][pl];
            if sum > sample {
                return v;
            }
        }
        Vertex::pass()
    }
}
