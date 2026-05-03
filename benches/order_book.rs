use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use latios_lob::{Order, OrderBook, OrderId, Side};
use std::hint::black_box;

const BID_BASE: u64 = 100_000_000; // 100.000000 in 6-decimal fixed-point
const ASK_BASE: u64 = 101_000_000; // 101.000000
const TICK: u64 = 100_000; // 0.100000 per level

fn make_orders(n: u64, levels: u64) -> Vec<Order> {
    (0..n)
        .map(|i| {
            if i < n / 2 {
                Order::new(i, BID_BASE - (i % levels) * TICK, 100, Side::Bid, i)
            } else {
                Order::new(i, ASK_BASE + (i % levels) * TICK, 100, Side::Ask, i)
            }
        })
        .collect()
}

/// Baseline: mixed add + cancel across 10 levels, ~33% cancel rate.
fn bench_mixed_workload(c: &mut Criterion) {
    const N: u64 = 1_000;
    const LEVELS: u64 = 10;

    let orders = make_orders(N, LEVELS);
    let cancel_ids: Vec<OrderId> = (0..N).step_by(3).collect();

    c.bench_function("mixed_add_cancel_1k", |b| {
        b.iter_batched(
            OrderBook::new,
            |mut book| {
                for order in &orders {
                    black_box(book.add_order(*order));
                }
                for &id in &cancel_ids {
                    black_box(book.cancel_order(id));
                }
                black_box(book.best_bid());
                black_box(book.best_ask());
            },
            BatchSize::SmallInput,
        )
    });
}

/// Add-only: isolates insertion path without cancel overhead.
fn bench_add_only(c: &mut Criterion) {
    const N: u64 = 1_000;
    const LEVELS: u64 = 10;

    let orders = make_orders(N, LEVELS);

    c.bench_function("add_only_1k", |b| {
        b.iter_batched(
            OrderBook::new,
            |mut book| {
                for order in &orders {
                    black_box(book.add_order(*order));
                }
            },
            BatchSize::SmallInput,
        )
    });
}

/// Cancel-only: book pre-populated in setup (not timed), measures only cancel path.
fn bench_cancel_only(c: &mut Criterion) {
    const N: u64 = 1_000;
    const LEVELS: u64 = 10;

    let orders = make_orders(N, LEVELS);
    let cancel_ids: Vec<OrderId> = (0..N).collect();

    c.bench_function("cancel_only_1k", |b| {
        b.iter_batched(
            || {
                let mut book = OrderBook::new();
                for order in &orders {
                    book.add_order(*order);
                }
                book
            },
            |mut book| {
                for &id in &cancel_ids {
                    black_box(book.cancel_order(id));
                }
            },
            BatchSize::SmallInput,
        )
    });
}

/// match_order: book pre-populated with asks, timed phase fires crossing bids.
/// Each taker bid is priced above all ask levels so it always hits the best ask.
fn bench_match_order(c: &mut Criterion) {
    const N: u64 = 1_000;
    const LEVELS: u64 = 10;

    let ask_orders: Vec<Order> = (0..N)
        .map(|i| Order::new(i, ASK_BASE + (i % LEVELS) * TICK, 100, Side::Ask, i))
        .collect();

    // Takers priced above the highest ask level so they always cross.
    let taker_bids: Vec<Order> = (0..N)
        .map(|i| Order::new(N + i, ASK_BASE + LEVELS * TICK, 1, Side::Bid, i))
        .collect();

    c.bench_function("match_order_1k", |b| {
        b.iter_batched(
            || {
                let mut book = OrderBook::new();
                for order in &ask_orders {
                    book.add_order(*order);
                }
                book
            },
            |mut book| {
                for taker in &taker_bids {
                    black_box(book.match_order(*taker));
                }
            },
            BatchSize::SmallInput,
        )
    });
}

/// Depth scaling: 1 000 adds across varying numbers of price levels.
/// Shows how binary search on the levels Vec scales with depth.
fn bench_add_depth_scaling(c: &mut Criterion) {
    const N: u64 = 1_000;

    let mut group = c.benchmark_group("add_depth_scaling");
    for &levels in &[1u64, 2, 5, 10, 20, 50, 100] {
        let orders = make_orders(N, levels);
        group.bench_with_input(BenchmarkId::from_parameter(levels), &orders, |b, orders| {
            b.iter_batched(
                OrderBook::new,
                |mut book| {
                    for order in orders {
                        black_box(book.add_order(*order));
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

/// best_bid / best_ask on a populated book — hot path for market data feeds.
fn bench_best_bid_ask(c: &mut Criterion) {
    const N: u64 = 1_000;
    const LEVELS: u64 = 10;

    let mut book = OrderBook::new();
    for order in make_orders(N, LEVELS) {
        book.add_order(order);
    }

    c.bench_function("best_bid_ask", |b| {
        b.iter(|| {
            black_box(book.best_bid());
            black_box(book.best_ask());
        })
    });
}

criterion_group!(
    benches,
    bench_mixed_workload,
    bench_add_only,
    bench_cancel_only,
    bench_match_order,
    bench_add_depth_scaling,
    bench_best_bid_ask,
);
criterion_main!(benches);
