use std::collections::BTreeMap;

use crate::types::{Order, OrderId, Price, Quantity};

/// A single price level — naive, intentionally cache-unfriendly.
///
/// BTreeMap<OrderId, Order> means every insertion/removal pointer-chases
/// through heap-allocated tree nodes. Each node is likely on a separate
/// cache line, so a lookup touches O(log n) cold lines.
///
/// TODO: replace with Vec<Order> backed by the arena allocator.
#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: Price,
    orders: BTreeMap<OrderId, Order>,
}

impl PriceLevel {
    pub fn new(_price: Price) -> Self {
        unimplemented!()
    }

    pub fn add(&mut self, _order: Order) {
        unimplemented!()
    }

    /// Removes the order with `id`. Returns it if found.
    pub fn remove(&mut self, _id: OrderId) -> Option<Order> {
        unimplemented!()
    }

    pub fn total_quantity(&self) -> Quantity {
        unimplemented!()
    }

    pub fn is_empty(&self) -> bool {
        unimplemented!()
    }

    pub fn order_count(&self) -> usize {
        unimplemented!()
    }
}

impl PartialEq for PriceLevel {
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
    }
}

impl Eq for PriceLevel {}

impl PartialOrd for PriceLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriceLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.price.cmp(&other.price)
    }
}
