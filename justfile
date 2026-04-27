default:
    just --list

build:
    cargo build 

test:
    cargo test

bench:
    cargo build --release
    cargo bench --benches
    perf stat -e cycles,instructions,L1-dcache-load-misses,L1-dcache-loads,branch-misses,branches,context-switches ./target/release/benchmark-harness

