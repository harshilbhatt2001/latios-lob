use std::collections::HashMap;

use crate::price_level::PriceLevel;
use crate::types::{Order, OrderId, OrderResult, Price, Side};

/// Naive order book — intentionally cache-unfriendly.
///
/// HashMap<Price, PriceLevel> means every price lookup is a heap allocation
/// + hash probe. Combined with PriceLevel's inner BTreeMap, every operation
/// pointer-chases through multiple heap-allocated nodes.
///
/// TODO: replace with sorted Vec<PriceLevel> + binary_search.
#[derive(Debug)]
pub struct OrderBook {
    bids: HashMap<Price, PriceLevel>,
    asks: HashMap<Price, PriceLevel>,
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: HashMap::<Price, PriceLevel>::new(),
            asks: HashMap::<Price, PriceLevel>::new(),
        }
    }

    /// Insert a new resting order. Returns Added or an error variant.
    pub fn add_order(&mut self, order: Order) -> OrderResult {
        let id = order.id;
        let book = match order.side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        book.entry(order.price)
            .or_insert_with(|| PriceLevel::new(order.price))
            .add(order);

        OrderResult::Added(id)
    }

    /// Remove a resting order by id. Returns Cancelled or NotFound.
    pub fn cancel_order(&mut self, id: OrderId) -> OrderResult {
        let bid_price = {
            let mut found = None;
            for (&p, level) in &mut self.bids {
                if level.remove(id).is_some() {
                    found = Some(p);
                    break;
                }
            }
            found
        };
        if let Some(p) = bid_price {
            if self.bids[&p].is_empty() { self.bids.remove(&p); }
            return OrderResult::Cancelled(id);
        }

        let ask_price = {
            let mut found = None;
            for (&p, level) in &mut self.asks {
                if level.remove(id).is_some() {
                    found = Some(p);
                    break;
                }
            }
            found
        };
        if let Some(p) = ask_price {
            if self.asks[&p].is_empty() { self.asks.remove(&p); }
            return OrderResult::Cancelled(id);
        }

        OrderResult::NotFound(id)
    }

    /// O(n) over price levels — scans all HashMap keys to find max.
    /// TODO: O(1) once replaced with sorted Vec (best bid = last element).
    pub fn best_bid(&self) -> Option<Price> {
        if self.bids.is_empty() {
            return Option::None;
        }

        self.bids.keys().max().copied()
    }

    /// O(n) over price levels — scans all HashMap keys to find min.
    /// TODO: O(1) once replaced with sorted Vec (best ask = first element).
    pub fn best_ask(&self) -> Option<Price> {
        if self.asks.is_empty() {
            return Option::None;
        }

        self.asks.keys().min().copied()
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

    // --- private helpers ---------------------------------------------------

    /// HashMap lookup for the given side and price.
    fn _level_for(&self, _side: Side, _price: Price) -> Option<&PriceLevel> {
        unimplemented!()
    }

    fn _level_for_mut(&mut self, _side: Side, _price: Price) -> Option<&mut PriceLevel> {
        unimplemented!()
    }

    // --- Vec optimisation stubs --------------------------------------------

    /// TODO: replace HashMap entry() calls with this once migrated to Vec<PriceLevel>.
    fn _find_or_insert_level(_levels: &mut Vec<PriceLevel>, _price: Price) -> &mut PriceLevel {
        unimplemented!()
    }

    /// TODO: call after cancels to keep Vec compact once migrated.
    fn _remove_empty_levels(_levels: &mut Vec<PriceLevel>) {
        unimplemented!()
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
