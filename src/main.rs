mod benchmark;
mod board;
mod fast_random;
mod gammas;
mod hash;
mod nat_map;
mod nat_set;
mod perf_counter;
mod sampler;
mod snapshot_test;
mod types;

use crate::benchmark::Benchmark;

fn main() {
    let mut bench = Benchmark::new();

    println!("{}", bench.run(10000, Some(1150865)));
    println!("{}", bench.run(100000, Some(11508282)));
    // println!("{}", bench.run(100000));
    // println!("{}", bench.run(100000));
    // println!("{}", bench.run(100000));
    // println!("{}", bench.run(100000));
}
