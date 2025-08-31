# go_game_board

High-performance Go/Baduk/Weiqi board representation and Monte Carlo playouts in Rust.

Based on the libEGo library's proven algorithms and data structures.

## Features

- Fast board representation with Zobrist hashing
- Efficient move generation and validation
- Monte Carlo playout engine with pattern-based move sampling
- Second-order pseudo liberty tracking - allows finding liberty of groups in atari
- Ko detection and super-ko via positional hashing
- Performance counters for benchmarking

## Usage

```rust
use go_game_board::{Board, Player, Vertex};

let mut board = Board::new();
board.clear();

// Play a move
let vertex = Vertex::from_coords(3, 3);
board.play_legal(Player::Black, vertex);

// Run benchmark
use go_game_board::Benchmark;
let mut bench = Benchmark::new();
println!("{}", bench.run(10000, None));
```

## Performance

This library is optimized for high-performance Monte Carlo tree search (MCTS) applications.
On 2GHz CPU it achieves ~3M moves per second
(compared to libEGo's 3.6M moves per second - the fastest implementation in the world)

Run benchmarks with:
```bash
cargo test --release --test benchmark_test
```

## Dependencies

- `go_game_types` - Shared type definitions for Go game libraries
- `arrayvec` - Stack-allocated vectors for performance
- `lazy_static` - Lazy static initialization
- `perf-event` - Performance counter support (Linux)

## License

Apache-2.0
