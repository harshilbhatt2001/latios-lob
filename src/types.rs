pub type OrderId = u64;
pub type Price = u64; // fixed-point, e.g. 1_000_000 == $1.000000
pub type Quantity = u64;
pub type Timestamp = u64; // nanoseconds since epoch

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub id: OrderId,
    pub price: Price,
    pub quantity: Quantity,
    pub side: Side,
    pub timestamp: Timestamp,
}

impl Order {
    pub fn new(
        id: OrderId,
        price: Price,
        quantity: Quantity,
        side: Side,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            id,
            price,
            quantity,
            side,
            timestamp,
        }
    }

    pub fn is_filled(&self) -> bool {
        self.quantity == 0
    }
}

#[derive(Debug)]
pub struct Trade {
    pub price: Price,
    pub quantity: Quantity,
    pub maker_id: OrderId,
    pub taker_id: OrderId,
}

/// Result of attempting to match or modify an order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderResult {
    Added(OrderId),
    Cancelled(OrderId),
    NotFound(OrderId),
    PartialFill {
        id: OrderId,
        filled: Quantity,
        remaining: Quantity,
    },
    FullFill(OrderId),
}

#[cfg(test)]
mod type_tests {
    use super::*;

    #[test]
    fn order_is_64_bytes() {
        assert_eq!(size_of::<Order>(), 64);
    }
}
