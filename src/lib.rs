pub mod arena;
pub mod order_book;
pub mod price_level;
pub mod types;

pub use order_book::OrderBook;
pub use types::{Order, OrderId, OrderResult, Price, Quantity, Side, Timestamp};

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
