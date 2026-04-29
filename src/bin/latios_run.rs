use latios_lob::{Order, OrderBook, Side};
use std::env;
use std::hint::black_box;
use std::time::Instant;

fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[(sorted.len() * p).saturating_sub(1) / 100]
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100_000);
    let cancel_rate: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40);

    const BID_BASE: u64 = 100_000_000; // 100.000000
    const ASK_BASE: u64 = 101_000_000; // 101.000000
    const TICK: u64 = 100_000; // 0.100000 per level
    const LEVELS: u64 = 20;

    let orders: Vec<Order> = (0..n as u64)
        .map(|i| {
            if i % 2 == 0 {
                Order::new(i, BID_BASE - (i % LEVELS) * TICK, 100, Side::Bid, i)
            } else {
                Order::new(i, ASK_BASE + (i % LEVELS) * TICK, 100, Side::Ask, i)
            }
        })
        .collect();

    let cancel_ids: Vec<u64> = (0..n as u64).filter(|i| i % 100 < cancel_rate).collect();

    let n_adds = orders.len();
    let n_cancels = cancel_ids.len();
    let total_ops = n_adds + n_cancels;

    let mut add_ns: Vec<u64> = Vec::with_capacity(n_adds);
    let mut cancel_ns: Vec<u64> = Vec::with_capacity(n_cancels);

    let wall_start = Instant::now();

    let mut book = OrderBook::new();
    for order in &orders {
        let t0 = Instant::now();
        black_box(book.add_order(order.clone()));
        add_ns.push(t0.elapsed().as_nanos() as u64);
    }
    for &id in &cancel_ids {
        let t0 = Instant::now();
        black_box(book.cancel_order(id));
        cancel_ns.push(t0.elapsed().as_nanos() as u64);
    }

    let elapsed = wall_start.elapsed();

    // Combined latency distribution
    let mut all_ns: Vec<u64> = Vec::with_capacity(total_ops);
    all_ns.extend_from_slice(&add_ns);
    all_ns.extend_from_slice(&cancel_ns);
    all_ns.sort_unstable();

    add_ns.sort_unstable();
    cancel_ns.sort_unstable();

    let mean = all_ns.iter().sum::<u64>() / all_ns.len() as u64;
    let p50 = percentile(&all_ns, 50);
    let p99 = percentile(&all_ns, 99);
    let p999 = percentile(&all_ns, 99); // reuse slot; extend later if needed

    let add_mean = add_ns.iter().sum::<u64>() / add_ns.len() as u64;
    let add_p99 = percentile(&add_ns, 99);
    let cancel_mean = cancel_ns.iter().sum::<u64>() / cancel_ns.len() as u64;
    let cancel_p99 = percentile(&cancel_ns, 99);

    let throughput_mops = total_ops as f64 / elapsed.as_secs_f64() / 1_000_000.0;
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    let best_bid = book.best_bid();
    let best_ask = book.best_ask();
    let _ = p999;

    println!(
        "\
=== latios-lob ===
ops        {total_ops}  (adds: {n_adds}, cancels: {n_cancels})
elapsed    {elapsed_ms:.3} ms
throughput {throughput_mops:.3} Mops/s

             mean      p50      p99
all      {mean:>7} ns {p50:>7} ns {p99:>7} ns
add      {add_mean:>7} ns           {add_p99:>7} ns
cancel   {cancel_mean:>7} ns           {cancel_p99:>7} ns

book     best_bid={best_bid:?}  best_ask={best_ask:?}"
    );
}
