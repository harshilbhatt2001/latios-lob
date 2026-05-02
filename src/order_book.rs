use ahash::AHashMap;

use crate::price_level::PriceLevel;
use crate::types::{Order, OrderId, OrderResult, Price, Side};

// Packs (price: u32, vec-index: u31, side: 1 bit) into 8 bytes.
// bit 31 of idx_side = Side (0 = Bid, 1 = Ask); bits 0..30 = index.
#[derive(Clone, Copy, Debug)]
struct OrderMeta {
    price: u32,
    idx_side: u32,
}

impl OrderMeta {
    #[inline(always)]
    fn new(price: Price, idx: usize, side: Side) -> Self {
        let side_bit = match side {
            Side::Bid => 0u32,
            Side::Ask => 1u32 << 31,
        };
        Self { price: price as u32, idx_side: idx as u32 | side_bit }
    }

    #[inline(always)]
    fn price(self) -> Price { self.price as Price }
    #[inline(always)]
    fn idx(self) -> usize { (self.idx_side & 0x7FFF_FFFF) as usize }
    #[inline(always)]
    fn side(self) -> Side {
        if self.idx_side >> 31 == 0 { Side::Bid } else { Side::Ask }
    }
    #[inline(always)]
    fn set_idx(&mut self, idx: usize) {
        self.idx_side = (self.idx_side & (1 << 31)) | idx as u32;
    }
}

#[derive(Debug)]
pub struct OrderBook {
    bids: Vec<PriceLevel>,
    asks: Vec<PriceLevel>,
    order_metadata: AHashMap<OrderId, OrderMeta>,
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: Vec::<PriceLevel>::new(),
            asks: Vec::<PriceLevel>::new(),
            order_metadata: AHashMap::new(),
        }
    }

    /// Insert a new resting order. Returns Added or an error variant.
    pub fn add_order(&mut self, order: Order) -> OrderResult {
        let id = order.id;
        let price = order.price;
        let side = order.side;
        let levels = match order.side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        let level_idx = match levels.binary_search_by_key(&price, |level| level.price) {
            Ok(idx) => idx,
            Err(idx) => {
                levels.insert(idx, PriceLevel::new(price));
                idx
            }
        };

        let order_idx = levels[level_idx].add(order);
        self.order_metadata.insert(id, OrderMeta::new(price, order_idx, side));

        OrderResult::Added(id)
    }

    /// Remove a resting order by id. Returns Cancelled or NotFound.
    pub fn cancel_order(&mut self, id: OrderId) -> OrderResult {
        if let Some(meta) = self.order_metadata.remove(&id) {
            let vec_idx = meta.idx();
            let levels = match meta.side() {
                Side::Ask => &mut self.asks,
                Side::Bid => &mut self.bids,
            };

            if let Ok(level_idx) = levels.binary_search_by_key(&meta.price(), |l| l.price) {
                let (_, moved_id) = levels[level_idx].remove_at(vec_idx);

                if let Some(m_id) = moved_id
                    && let Some(moved_meta) = self.order_metadata.get_mut(&m_id)
                {
                    moved_meta.set_idx(vec_idx);
                }

                if levels[level_idx].is_empty() {
                    levels.swap_remove(level_idx);
                    //levels.sort_unstable_by_key(|l| l.price);
                }
                return OrderResult::Cancelled(id);
            }
            OrderResult::Cancelled(id)
        } else {
            OrderResult::NotFound(id)
        }
    }

    ///  over price levels — scans to find max.
    pub fn best_bid(&self) -> Option<Price> {
        if self.bids.is_empty() {
            return Option::None;
        }

        Some(self.bids.last().unwrap().price)
    }

    ///  over price levels — scans to find min.
    pub fn best_ask(&self) -> Option<Price> {
        if self.asks.is_empty() {
            return Option::None;
        }

        Some(self.asks.first().unwrap().price)
    }

    /// Mid-price, or None if either side is empty.
    pub fn mid_price(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2),
            _ => None,
        }
    }

    /// Spread in fixed-point units, or None if either side is empty.
    pub fn spread(&self) -> Option<Price> {
        Some(self.best_ask()? - self.best_bid()?)
    }

    pub fn bid_depth(&self) -> usize {
        self.bids.len()
    }

    pub fn ask_depth(&self) -> usize {
        self.asks.len()
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod book_invariants {
    use super::*;
    use crate::types::{Order, OrderResult, Side};

    fn bid(id: OrderId, price: Price) -> Order {
        Order::new(id, price, 10, Side::Bid, id)
    }

    fn ask(id: OrderId, price: Price) -> Order {
        Order::new(id, price, 10, Side::Ask, id)
    }

    // --- empty book ----------------------------------------------------------

    #[test]
    fn empty_book_has_no_best_bid() {
        let book = OrderBook::new();
        assert_eq!(book.best_bid(), None);
    }

    #[test]
    fn empty_book_has_no_best_ask() {
        let book = OrderBook::new();
        assert_eq!(book.best_ask(), None);
    }

    #[test]
    fn empty_book_has_no_spread() {
        let book = OrderBook::new();
        assert_eq!(book.spread(), None);
    }

    // --- single order --------------------------------------------------------

    #[test]
    fn add_bid_sets_best_bid() {
        let mut book = OrderBook::new();
        book.add_order(bid(1, 100));
        assert_eq!(book.best_bid(), Some(100));
    }

    #[test]
    fn add_ask_sets_best_ask() {
        let mut book = OrderBook::new();
        book.add_order(ask(1, 200));
        assert_eq!(book.best_ask(), Some(200));
    }

    #[test]
    fn add_order_returns_added() {
        let mut book = OrderBook::new();
        assert_eq!(book.add_order(bid(1, 100)), OrderResult::Added(1));
    }

    // --- best price with multiple levels -------------------------------------

    #[test]
    fn best_bid_is_highest_price() {
        let mut book = OrderBook::new();
        book.add_order(bid(1, 99));
        book.add_order(bid(2, 101));
        book.add_order(bid(3, 100));
        assert_eq!(book.best_bid(), Some(101));
    }

    #[test]
    fn best_ask_is_lowest_price() {
        let mut book = OrderBook::new();
        book.add_order(ask(1, 201));
        book.add_order(ask(2, 199));
        book.add_order(ask(3, 200));
        assert_eq!(book.best_ask(), Some(199));
    }

    // --- spread invariant ----------------------------------------------------

    /// The book must never be crossed: best_bid < best_ask always.
    #[test]
    fn spread_is_non_negative() {
        let mut book = OrderBook::new();
        book.add_order(bid(1, 100));
        book.add_order(ask(2, 200));
        let bid = book.best_bid().unwrap();
        let ask = book.best_ask().unwrap();
        assert!(bid < ask, "crossed book: bid {bid} >= ask {ask}");
    }

    // --- cancel --------------------------------------------------------------

    #[test]
    fn cancel_only_bid_clears_best_bid() {
        let mut book = OrderBook::new();
        book.add_order(bid(1, 100));
        book.cancel_order(1);
        assert_eq!(book.best_bid(), None);
    }

    #[test]
    fn cancel_best_bid_reveals_next_level() {
        let mut book = OrderBook::new();
        book.add_order(bid(1, 100));
        book.add_order(bid(2, 99));
        book.cancel_order(1);
        assert_eq!(book.best_bid(), Some(99));
    }

    #[test]
    fn cancel_returns_cancelled() {
        let mut book = OrderBook::new();
        book.add_order(bid(1, 100));
        assert_eq!(book.cancel_order(1), OrderResult::Cancelled(1));
    }

    #[test]
    fn cancel_unknown_id_returns_not_found() {
        let mut book = OrderBook::new();
        assert_eq!(book.cancel_order(999), OrderResult::NotFound(999));
    }

    // --- depth ---------------------------------------------------------------

    #[test]
    fn bid_depth_counts_price_levels() {
        let mut book = OrderBook::new();
        book.add_order(bid(1, 100));
        book.add_order(bid(2, 100)); // same level, depth still 1
        book.add_order(bid(3, 99));
        assert_eq!(book.bid_depth(), 2);
    }

    #[test]
    fn ask_depth_counts_price_levels() {
        let mut book = OrderBook::new();
        book.add_order(ask(1, 200));
        book.add_order(ask(2, 201));
        assert_eq!(book.ask_depth(), 2);
    }
}

#[cfg(test)]
mod edge_cases {
    use super::*;
    use crate::types::{Order, OrderResult, Side};

    // Minimal LCG — no external deps needed.
    fn next_rand(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state >> 33
    }

    // ── 1. Mass-cancel: 1 000 random orders, cancel every one → empty book ──

    #[test]
    fn mass_cancel_leaves_book_empty() {
        let mut book = OrderBook::new();
        let mut rng = 0xDEAD_BEEF_CAFE_u64;
        const N: u64 = 1_000;

        for id in 1..=N {
            // Bids in [100,119], asks in [130,149] — book stays uncrossed by input.
            let (price, side) = if next_rand(&mut rng) % 2 == 0 {
                (100 + next_rand(&mut rng) % 20, Side::Bid)
            } else {
                (130 + next_rand(&mut rng) % 20, Side::Ask)
            };
            book.add_order(Order::new(id, price, 1, side, id));
        }

        for id in 1..=N {
            book.cancel_order(id);
        }

        assert_eq!(book.bid_depth(), 0, "bid levels remain after mass cancel");
        assert_eq!(book.ask_depth(), 0, "ask levels remain after mass cancel");
        assert_eq!(book.best_bid(), None);
        assert_eq!(book.best_ask(), None);
        // Metadata map must also be drained — no ghost entries.
        assert_eq!(
            book.order_metadata.len(),
            0,
            "stale metadata after mass cancel"
        );
    }

    // ── 2. Crossed-book invariant: best_bid < best_ask after every operation ──

    #[test]
    fn book_never_crosses_under_random_ops() {
        let mut book = OrderBook::new();
        let mut rng = 0xCAFE_BABE_1234_u64;
        // live_ids tracks orders still in the book so we can cancel random ones.
        let mut live_ids: Vec<OrderId> = Vec::new();
        let mut next_id: OrderId = 1;

        for _ in 0..3_000 {
            match next_rand(&mut rng) % 3 {
                0 => {
                    // add bid: prices strictly below 150
                    let price = 100 + next_rand(&mut rng) % 50;
                    book.add_order(Order::new(next_id, price, 1, Side::Bid, next_id));
                    live_ids.push(next_id);
                    next_id += 1;
                }
                1 => {
                    // add ask: prices strictly above 149
                    let price = 150 + next_rand(&mut rng) % 50;
                    book.add_order(Order::new(next_id, price, 1, Side::Ask, next_id));
                    live_ids.push(next_id);
                    next_id += 1;
                }
                _ => {
                    // cancel a random live order
                    if !live_ids.is_empty() {
                        let idx = (next_rand(&mut rng) as usize) % live_ids.len();
                        let id = live_ids.swap_remove(idx);
                        book.cancel_order(id);
                    }
                }
            }

            if let (Some(bid), Some(ask)) = (book.best_bid(), book.best_ask()) {
                assert!(
                    bid < ask,
                    "crossed book after {} ops: best_bid={bid} >= best_ask={ask}",
                    next_id - 1
                );
            }
        }
    }

    // ── 3. Cancelling a non-existent order_id returns NotFound, not a panic ──

    #[test]
    fn cancel_nonexistent_id_returns_not_found() {
        let mut book = OrderBook::new();
        assert_eq!(book.cancel_order(0), OrderResult::NotFound(0));
        assert_eq!(book.cancel_order(9999), OrderResult::NotFound(9999));
        assert_eq!(book.cancel_order(u64::MAX), OrderResult::NotFound(u64::MAX));
    }

    #[test]
    fn cancel_already_cancelled_returns_not_found() {
        // Cancelling an order a second time must not return Cancelled — the order
        // no longer exists, so NotFound is the correct response.
        // This also guards against a stale-metadata bug where order_metadata is
        // never drained on cancel, causing a spurious Cancelled on a repeat call.
        let mut book = OrderBook::new();
        book.add_order(Order::new(1, 100, 10, Side::Bid, 1));
        assert_eq!(book.cancel_order(1), OrderResult::Cancelled(1));
        assert_eq!(
            book.cancel_order(1),
            OrderResult::NotFound(1),
            "second cancel of the same id must be NotFound"
        );
    }

    // ── 4. add_order with a duplicate order_id on the same level ─────────────
    //
    // Design decision: duplicate order_ids are *silently appended* to the same
    // price level (no Err/rejection). Each add_order call returns Added(id).
    // Callers are responsible for ensuring id uniqueness; the book does not
    // deduplicate.  cancel_order will remove the first match by FIFO order.

    #[test]
    fn duplicate_order_id_appends_to_same_level() {
        let mut book = OrderBook::new();
        let r1 = book.add_order(Order::new(1, 100, 10, Side::Bid, 0));
        let r2 = book.add_order(Order::new(1, 100, 20, Side::Bid, 1));

        assert_eq!(r1, OrderResult::Added(1));
        assert_eq!(r2, OrderResult::Added(1));
        // Still one price level, not two.
        assert_eq!(book.bid_depth(), 1);
        assert_eq!(book.best_bid(), Some(100));
    }
}
