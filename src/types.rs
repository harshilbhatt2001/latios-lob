pub type OrderId = u64;
pub type Price = u64; // fixed-point, e.g. 1_000_000 == $1.000000
pub type Quantity = u64;
pub type Timestamp = u64; // nanoseconds since epoch

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

/// Naive order representation — no alignment padding yet.
/// TODO: add #[repr(C)] + explicit 64-byte padding to prevent false sharing.
#[derive(Debug, Clone)]
pub struct Order {
    pub id: OrderId,
    pub price: Price,
    pub quantity: Quantity,
    pub side: Side,
    pub timestamp: Timestamp,
}

impl Order {
    pub fn new(_id: OrderId, _price: Price, _quantity: Quantity, _side: Side, _timestamp: Timestamp) -> Self {
        unimplemented!()
    }

    pub fn is_filled(&self) -> bool {
        unimplemented!()
    }
}

/// Result of attempting to match or modify an order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderResult {
    Added(OrderId),
    Cancelled(OrderId),
    NotFound(OrderId),
    PartialFill { id: OrderId, filled: Quantity, remaining: Quantity },
    FullFill(OrderId),
}
