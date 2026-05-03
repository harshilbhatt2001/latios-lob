use crate::types::{Order, OrderId, Price, Quantity};

#[derive(Debug)]
pub struct PriceLevel {
    pub price: Price,
    orders: Vec<Order>,
    total_volume: Quantity,
}

impl PriceLevel {
    pub fn new(price: Price) -> Self {
        Self {
            price,
            orders: Vec::<Order>::new(),
            total_volume: 0,
        }
    }

    pub fn add(&mut self, order: Order) -> usize {
        let index = self.orders.len();
        self.total_volume += order.quantity;
        self.orders.push(order);
        index
    }

    pub fn remove_at(&mut self, idx: usize) -> (Order, Option<OrderId>) {
        self.total_volume -= self.orders[idx].quantity;
        let removed = self.orders.swap_remove(idx);

        let moved_id = self.orders.get(idx).map(|o| o.id);
        (removed, moved_id)
    }

    pub fn total_quantity(&self) -> Quantity {
        self.total_volume
    }

    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    pub fn order_count(&self) -> usize {
        self.orders.len()
    }

    pub fn first(&mut self) -> &mut Order {
        &mut self.orders[0]
    }

    /// Match `available` qty against the front order.
    /// Returns `(filled, maker_id, maker_exhausted)`.
    pub fn match_front(&mut self, available: Quantity) -> (Quantity, OrderId, bool) {
        let front = &mut self.orders[0];
        let qty = available.min(front.quantity);
        front.quantity -= qty;
        self.total_volume -= qty;
        (qty, front.id, front.quantity == 0)
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
