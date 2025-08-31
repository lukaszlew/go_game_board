use crate::board::Board;
use crate::fast_random::FastRandom;
use crate::gammas::Gammas;
use crate::perf_counter::PerfCounter;
use crate::sampler::Sampler;
use crate::types::{Player, PlayerMap};
use std::time::Instant;

pub struct Benchmark {
    empty_board: Board,
    board: Board,
    random: FastRandom,
    gammas: Gammas,
    move_count: usize,
}

impl Benchmark {
    pub fn new() -> Self {
        let mut empty_board = Board::new();
        empty_board.clear();

        Benchmark {
            empty_board: empty_board.clone(),
            board: empty_board,
            random: FastRandom::new(123),
            gammas: Gammas::new(),
            move_count: 0,
        }
    }

    fn do_playouts(&mut self, playout_cnt: usize, win_cnt: &mut PlayerMap<usize>) {
        let mut sampler = Sampler::new(&self.board, &self.gammas);

        for _i in 0..playout_cnt {
            self.board.load(&self.empty_board);
            sampler.new_playout(&self.board, &self.gammas);

            while !self.board.both_player_pass() {
                let pl = self.board.act_player();
                let v = sampler.sample_move(&self.board, &mut self.random);

                self.board.play_legal(pl, v);
                sampler.move_played(&self.board, &self.gammas);
            }

            let winner = self.board.playout_winner();
            win_cnt[winner] += 1;
            self.move_count += self.board.move_count();
        }
    }

    pub fn run(&mut self, playout_cnt: usize, expected_moves: Option<usize>) -> String {
        self.move_count = 0;
        self.random = FastRandom::new(123);

        let mut win_cnt = PlayerMap::<usize>::new();
        win_cnt[Player::Black] = 0;
        win_cnt[Player::White] = 0;

        // Initialize perf counter
        let mut perf_counter = PerfCounter::new();

        // Start both timing methods
        perf_counter.start();
        let start = Instant::now();

        self.do_playouts(playout_cnt, &mut win_cnt);

        // Stop timing and read counter
        let duration = start.elapsed();
        // Stop and then read the perf counter
        perf_counter.stop();
        let perf_cycles = perf_counter.read();

        let seconds_total = duration.as_secs_f32();
        let playouts_finished = win_cnt[Player::Black] + win_cnt[Player::White];
        let kpps = (playout_cnt as f32) / seconds_total / 1000.0;

        // Try to read CPU frequency
        let cpu_freq_ghz = get_cpu_frequency_ghz();
        let total_clock_cycles = seconds_total as f64 * cpu_freq_ghz * 1e9;
        let cc_per_move = total_clock_cycles / self.move_count as f64;

        // Calculate CC/move from perf counter if valid
        let perf_cc_per_move = if perf_counter.is_valid() {
            format!("{:.1}", perf_cycles as f64 / self.move_count as f64)
        } else {
            "N/A".to_string()
        };

        let avg_moves = self.move_count as f32 / playouts_finished as f32;

        // Assert expected move count if provided
        assert_eq!(
            expected_moves.unwrap_or(self.move_count as usize),
            self.move_count as usize
        );

        format!(
            "\n{} playouts \n\
             in {:.6} seconds => {:.3} kpps\n\
             CC/move (time*freq, perf counter): {:.1} / {}  @  CPU freq: {:.3} GHz\n\
             {}/{} (black wins / white wins)\n\
             AVG moves/playout = {:.6}",
            playout_cnt,
            seconds_total,
            kpps,
            cc_per_move,
            perf_cc_per_move,
            cpu_freq_ghz,
            win_cnt[Player::Black],
            win_cnt[Player::White],
            avg_moves
        )
    }
}

fn get_cpu_frequency_ghz() -> f64 {
    // Try to read current CPU frequency from /sys
    if let Ok(contents) =
        std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq")
    {
        if let Ok(freq_khz) = contents.trim().parse::<f64>() {
            return freq_khz / 1_000_000.0; // Convert kHz to GHz
        }
    }

    // Fallback
    if let Ok(contents) =
        std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_cur_freq")
    {
        if let Ok(freq_khz) = contents.trim().parse::<f64>() {
            return freq_khz / 1_000_000.0;
        }
    }

    // Default fallback
    eprintln!("Warning: Could not read CPU frequency, assuming 1.0 GHz");
    1.0
}
