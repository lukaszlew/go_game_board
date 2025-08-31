use go_game_board::Benchmark;

#[test]
fn test_benchmark_10k() {
    let mut bench = Benchmark::new();
    let result = bench.run(10000, Some(1150865));
    println!("{}", result);
}

#[test]
fn test_benchmark_100k() {
    let mut bench = Benchmark::new();
    let result = bench.run(100000, Some(11508282));
    println!("{}", result);
}

#[test]
#[ignore] // Run with cargo test -- --ignored
fn benchmark_performance() {
    let mut bench = Benchmark::new();
    println!("{}", bench.run(100000, None));
    println!("{}", bench.run(100000, None));
    println!("{}", bench.run(100000, None));
    println!("{}", bench.run(100000, None));
}