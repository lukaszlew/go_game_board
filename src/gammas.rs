use crate::hash::{Hash3x3, Hash3x3Map};
use crate::types::{Nat, Player, PlayerMap};

pub const GAMMAS_ACCURACY: f64 = 1.0e-10;

pub struct Gammas {
    gammas: Hash3x3Map<PlayerMap<f64>>,
}

impl Gammas {
    pub fn new() -> Self {
        let mut gammas = Gammas {
            gammas: Hash3x3Map::new(),
        };
        gammas.reset_to_uniform();
        gammas
    }

    pub fn reset_to_uniform(&mut self) {
        for hash in Hash3x3::all() {
            for pl in Player::all() {
                self.gammas[hash][pl] = if hash.is_legal(pl) && !hash.is_eyelike(pl) {
                    1.0
                } else {
                    0.0
                };
            }
        }
    }

    pub fn get(&self, hash: Hash3x3, pl: Player) -> f64 {
        self.gammas[hash][pl]
    }
}
