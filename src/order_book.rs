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
pub struct OrderBook {
    bids: HashMap<Price, PriceLevel>,
    asks: HashMap<Price, PriceLevel>,
}

impl OrderBook {
    pub fn new() -> Self {
        unimplemented!()
    }

    /// Insert a new resting order. Returns Added or an error variant.
    pub fn add_order(&mut self, _order: Order) -> OrderResult {
        unimplemented!()
    }

    /// Remove a resting order by id. Returns Cancelled or NotFound.
    pub fn cancel_order(&mut self, _id: OrderId) -> OrderResult {
        unimplemented!()
    }

    /// O(1) — best bid is the highest bid price level.
    pub fn best_bid(&self) -> Option<Price> {
        unimplemented!()
    }

    /// O(1) — best ask is the lowest ask price level.
    pub fn best_ask(&self) -> Option<Price> {
        unimplemented!()
    }

    /// Mid-price, or None if either side is empty.
    pub fn mid_price(&self) -> Option<Price> {
        unimplemented!()
    }

    /// Spread in fixed-point units, or None if either side is empty.
    pub fn spread(&self) -> Option<Price> {
        unimplemented!()
    }

    pub fn bid_depth(&self) -> usize {
        unimplemented!()
    }

    pub fn ask_depth(&self) -> usize {
        unimplemented!()
    }

    // --- private helpers ---------------------------------------------------

    fn find_or_insert_level(_levels: &mut Vec<PriceLevel>, _price: Price) -> &mut PriceLevel {
        unimplemented!()
    }

    fn remove_empty_levels(_levels: &mut Vec<PriceLevel>) {
        unimplemented!()
    }

    /// Returns the side-appropriate Vec and a reference into it for `price`,
    /// located via binary_search.
    fn level_for(&self, _side: Side, _price: Price) -> Option<&PriceLevel> {
        unimplemented!()
    }

    fn level_for_mut(&mut self, _side: Side, _price: Price) -> Option<&mut PriceLevel> {
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
    use crate::types::{OrderResult, Side};

    fn bid(id: OrderId, price: Price) -> Order {
        Order { id, price, quantity: 10, side: Side::Bid, timestamp: id }
    }

    fn ask(id: OrderId, price: Price) -> Order {
        Order { id, price, quantity: 10, side: Side::Ask, timestamp: id }
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
