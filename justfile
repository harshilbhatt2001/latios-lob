default:
    just --list

build:
    cargo build 

release:
    cargo build --release

test:
    cargo test

bench:
    cargo bench --benches

