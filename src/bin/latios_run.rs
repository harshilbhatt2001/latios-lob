use clap::Parser;
use latios_lob::{Order, OrderBook, Side};
use std::hint::black_box;
use std::time::Instant;

#[derive(Parser)]
#[command(about = "latios-lob profiling harness")]
struct Args {
    /// Number of orders per iteration
    #[arg(short, long, default_value_t = 100_000)]
    n: usize,

    /// Cancel rate: orders per 100 that are cancelled (0–100)
    #[arg(short, long, default_value_t = 40)]
    cancel_rate: u64,

    /// Number of timing iterations (ignored in quiet mode)
    #[arg(short, long, default_value_t = 200)]
    iters: usize,

    /// Bare workload only — no timing, no output, no allocations.
    /// Use when running under perf stat so counters reflect only book ops.
    #[arg(short, long)]
    quiet: bool,
}

// p_milli is in thousandths: 500 = p50, 990 = p99, 999 = p99.9
fn percentile(sorted: &[u64], p_milli: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() * p_milli).saturating_sub(1) / 1000).min(sorted.len() - 1);
    sorted[idx]
}

fn main() {
    let args = Args::parse();

    const BID_BASE: u64 = 100_000_000; // 100.000000
    const ASK_BASE: u64 = 101_000_000; // 101.000000
    const TICK: u64 = 100_000; // 0.100000 per level
    const LEVELS: u64 = 20;

    let orders: Vec<Order> = (0..args.n as u64)
        .map(|i| {
            if i % 2 == 0 {
                Order::new(i, BID_BASE - (i % LEVELS) * TICK, 100, Side::Bid, i)
            } else {
                Order::new(i, ASK_BASE + (i % LEVELS) * TICK, 100, Side::Ask, i)
            }
        })
        .collect();

    let cancel_ids: Vec<u64> = (0..args.n as u64)
        .filter(|i| i % 100 < args.cancel_rate)
        .collect();

    if args.quiet {
        let mut book = OrderBook::new();
        for order in &orders {
            black_box(book.add_order(order.clone()));
        }
        for &id in &cancel_ids {
            black_box(book.cancel_order(id));
        }
        return;
    }

    let n_adds = orders.len();
    let n_cancels = cancel_ids.len();
    let total_ops = n_adds + n_cancels;
    let iters = args.iters;

    // Each entry is the mean ns/op for that iteration's phase, timed as a
    // whole-phase batch. Per-op Instant::now() costs ~25 ns itself, which
    // dominates short operations — batching amortizes that overhead away.
    let mut add_ns: Vec<u64> = Vec::with_capacity(iters);
    let mut cancel_ns: Vec<u64> = Vec::with_capacity(iters);

    let wall_start = Instant::now();
    let mut book = OrderBook::new();

    for _ in 0..iters {
        book = OrderBook::new();

        let t0 = Instant::now();
        for order in &orders {
            black_box(book.add_order(order.clone()));
        }
        add_ns.push(t0.elapsed().as_nanos() as u64 / n_adds as u64);

        if n_cancels > 0 {
            let t1 = Instant::now();
            for &id in &cancel_ids {
                black_box(book.cancel_order(id));
            }
            cancel_ns.push(t1.elapsed().as_nanos() as u64 / n_cancels as u64);
        }
    }

    let elapsed = wall_start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    let throughput_mops = (total_ops * iters) as f64 / elapsed.as_secs_f64() / 1_000_000.0;

    add_ns.sort_unstable();
    cancel_ns.sort_unstable();

    let add_mean = add_ns.iter().sum::<u64>() / add_ns.len() as u64;
    let add_p50 = percentile(&add_ns, 500);
    let add_p99 = percentile(&add_ns, 990);
    let add_p999 = percentile(&add_ns, 999);

    let (cancel_mean, cancel_p50, cancel_p99, cancel_p999) = if !cancel_ns.is_empty() {
        (
            cancel_ns.iter().sum::<u64>() / cancel_ns.len() as u64,
            percentile(&cancel_ns, 500),
            percentile(&cancel_ns, 990),
            percentile(&cancel_ns, 999),
        )
    } else {
        (0, 0, 0, 0)
    };

    let best_bid = book.best_bid();
    let best_ask = book.best_ask();

    println!(
        "\
=== latios-lob ({iters}×{n} ops) ===
ops/iter   {total_ops}  (adds: {n_adds}, cancels: {n_cancels})
elapsed    {elapsed_ms:.3} ms
throughput {throughput_mops:.3} Mops/s

           mean ns/op    p50      p99    p99.9
add        {add_mean:>7}      {add_p50:>7}  {add_p99:>7}  {add_p999:>7}
cancel     {cancel_mean:>7}      {cancel_p50:>7}  {cancel_p99:>7}  {cancel_p999:>7}

book     best_bid={best_bid:?}  best_ask={best_ask:?}",
        n = args.n
    );
}
