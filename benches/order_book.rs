use criterion::{black_box, criterion_group, criterion_main, Criterion};
use latios_lob::{Order, OrderId, OrderBook, Side};

/// Mixed add/cancel workload: 1 000 orders across 10 price levels per side,
/// ~33% cancel rate.  This is the baseline for the naive HashMap implementation
/// before the Vec migration.
fn bench_mixed_workload(c: &mut Criterion) {
    const N: u64 = 1_000;
    const LEVELS: u64 = 10;
    const BID_BASE: u64 = 100_000_000; // $100.000000 in 6-decimal fixed-point
    const ASK_BASE: u64 = 101_000_000; // $101.000000
    const TICK: u64 = 100_000;         // $0.100000 per level

    // Pre-generate order data outside the hot loop so we only measure book ops.
    let orders: Vec<Order> = (0..N)
        .map(|i| {
            if i < N / 2 {
                Order::new(i, BID_BASE - (i % LEVELS) * TICK, 100, Side::Bid, i)
            } else {
                Order::new(i, ASK_BASE + (i % LEVELS) * TICK, 100, Side::Ask, i)
            }
        })
        .collect();

    // Cancel every 3rd order (~33%).
    let cancel_ids: Vec<OrderId> = (0..N).step_by(3).collect();

    c.bench_function("mixed_add_cancel_1k_orders", |b| {
        b.iter(|| {
            let mut book = OrderBook::new();
            for order in &orders {
                black_box(book.add_order(order.clone()));
            }
            for &id in &cancel_ids {
                black_box(book.cancel_order(id));
            }
            black_box(book.best_bid());
            black_box(book.best_ask());
        })
    });
}

criterion_group!(benches, bench_mixed_workload);
criterion_main!(benches);
