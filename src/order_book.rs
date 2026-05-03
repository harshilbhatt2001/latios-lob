use ahash::AHashMap;

use crate::price_level::PriceLevel;
use crate::types::{Order, OrderId, OrderResult, Price, Side, Trade};

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
        Self {
            price: price as u32,
            idx_side: idx as u32 | side_bit,
        }
    }

    #[inline(always)]
    fn price(self) -> Price {
        self.price as Price
    }
    #[inline(always)]
    fn idx(self) -> usize {
        (self.idx_side & 0x7FFF_FFFF) as usize
    }
    #[inline(always)]
    fn side(self) -> Side {
        if self.idx_side >> 31 == 0 {
            Side::Bid
        } else {
            Side::Ask
        }
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
    trades: Vec<Trade>,
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: Vec::<PriceLevel>::new(),
            asks: Vec::<PriceLevel>::new(),
            order_metadata: AHashMap::new(),
            trades: Vec::<Trade>::new(),
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
        self.order_metadata
            .insert(id, OrderMeta::new(price, order_idx, side));

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

    pub fn match_order(&mut self, mut taker_order: Order) -> OrderResult {
        let crosses_spread = match taker_order.side {
            Side::Bid => self.best_ask().is_some_and(|ask| ask <= taker_order.price),
            Side::Ask => self.best_bid().is_some_and(|bid| bid >= taker_order.price),
        };

        if !crosses_spread {
            return self.add_order(taker_order);
        }

        while taker_order.quantity > 0 {
            let best_opp_price = match taker_order.side {
                Side::Bid => self.best_ask(),
                Side::Ask => self.best_bid(),
            };

            match best_opp_price {
                Some(price) => {
                    if (taker_order.side == Side::Bid && taker_order.price >= price)
                        || (taker_order.side == Side::Ask && taker_order.price <= price)
                    {
                        self.match_against_level(&mut taker_order, price);
                    } else {
                        break;
                    }
                }
                _ => break,
            };
        }

        if taker_order.quantity > 0 {
            return self.add_order(taker_order);
        }

        OrderResult::FullFill(taker_order.id)
    }

    fn match_against_level(&mut self, taker: &mut Order, price: Price) {
        // Find the opposing level index first, then drop the borrow so we can
        // freely access self.trades and self.order_metadata inside the loop.
        let level_idx = {
            let opp = match taker.side {
                Side::Bid => &self.asks,
                Side::Ask => &self.bids,
            };
            match opp.binary_search_by_key(&price, |l| l.price) {
                Ok(idx) => idx,
                Err(_) => return,
            }
        };

        loop {
            if taker.quantity == 0 {
                break;
            }

            // Short borrow: extract match info and release before touching other fields.
            let (match_qty, maker_id, fully_filled) = match taker.side {
                Side::Bid => self.asks[level_idx].match_front(taker.quantity),
                Side::Ask => self.bids[level_idx].match_front(taker.quantity),
            };

            self.trades.push(Trade {
                price,
                quantity: match_qty,
                maker_id,
                taker_id: taker.id,
            });
            taker.quantity -= match_qty;

            if fully_filled {
                self.order_metadata.remove(&maker_id);

                let (_, moved_id) = match taker.side {
                    Side::Bid => self.asks[level_idx].remove_at(0),
                    Side::Ask => self.bids[level_idx].remove_at(0),
                };

                if let Some(m_id) = moved_id
                    && let Some(meta) = self.order_metadata.get_mut(&m_id)
                {
                    meta.set_idx(0);
                }

                let now_empty = match taker.side {
                    Side::Bid => self.asks[level_idx].is_empty(),
                    Side::Ask => self.bids[level_idx].is_empty(),
                };
                if now_empty {
                    match taker.side {
                        Side::Bid => {
                            self.asks.remove(level_idx);
                        }
                        Side::Ask => {
                            self.bids.remove(level_idx);
                        }
                    }
                    break;
                }
            } else {
                // Partial fill of maker: taker is now exhausted.
                break;
            }
        }
    }

    ///  over price levels — scans to find max.
    #[inline(always)]
    pub fn best_bid(&self) -> Option<Price> {
        if self.bids.is_empty() {
            return Option::None;
        }

        Some(self.bids.last().unwrap().price)
    }

    ///  over price levels — scans to find min.
    #[inline(always)]
    pub fn best_ask(&self) -> Option<Price> {
        if self.asks.is_empty() {
            return Option::None;
        }

        Some(self.asks.first().unwrap().price)
    }

    /// Mid-price, or None if either side is empty.
    #[inline(always)]
    pub fn mid_price(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2),
            _ => None,
        }
    }

    /// Spread in fixed-point units, or None if either side is empty.
    #[inline(always)]
    pub fn spread(&self) -> Option<Price> {
        Some(self.best_ask()? - self.best_bid()?)
    }

    #[inline(always)]
    pub fn bid_depth(&self) -> usize {
        self.bids.len()
    }

    #[inline(always)]
    pub fn ask_depth(&self) -> usize {
        self.asks.len()
    }

    pub fn trades(&self) -> &[Trade] {
        &self.trades
    }

    pub fn drain_trades(&mut self) -> Vec<Trade> {
        std::mem::take(&mut self.trades)
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}
