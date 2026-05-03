use latios_lob::{Order, OrderBook, OrderId, OrderResult, Price, Side};

fn bid(id: OrderId, price: Price, qty: u64) -> Order {
    Order::new(id, price, qty, Side::Bid, id)
}
fn ask(id: OrderId, price: Price, qty: u64) -> Order {
    Order::new(id, price, qty, Side::Ask, id)
}

/// add_order, cancel_order, best prices, depth, spread.
#[test]
fn add_cancel_and_book_state() {
    // empty book
    let book = OrderBook::new();
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
    assert_eq!(book.spread(), None);

    // best prices with multiple levels
    let mut book = OrderBook::new();
    book.add_order(bid(1, 99, 10));
    book.add_order(bid(2, 101, 10));
    book.add_order(bid(3, 100, 10));
    book.add_order(ask(4, 201, 10));
    book.add_order(ask(5, 199, 10));
    book.add_order(ask(6, 200, 10));
    assert_eq!(book.best_bid(), Some(101));
    assert_eq!(book.best_ask(), Some(199));
    assert_eq!(book.bid_depth(), 3);
    assert_eq!(book.ask_depth(), 3);
    assert_eq!(book.spread(), Some(199 - 101));

    // cancel: result codes + best price update
    assert_eq!(book.cancel_order(2), OrderResult::Cancelled(2));
    assert_eq!(book.best_bid(), Some(100));
    assert_eq!(book.bid_depth(), 2);
    assert_eq!(book.cancel_order(999), OrderResult::NotFound(999));
    assert_eq!(book.cancel_order(2), OrderResult::NotFound(2)); // double-cancel

    // two orders at same level → one price level
    let mut book = OrderBook::new();
    book.add_order(bid(1, 100, 10));
    book.add_order(bid(2, 100, 20));
    assert_eq!(book.bid_depth(), 1);
    book.cancel_order(1);
    assert_eq!(book.best_bid(), Some(100)); // id=2 still resting
    book.cancel_order(2);
    assert_eq!(book.best_bid(), None);
}

/// match_order: non-crossing, full/partial fills, multi-level sweep, FIFO.
#[test]
fn matching() {
    // non-crossing: taker goes to resting side
    let mut book = OrderBook::new();
    book.add_order(ask(1, 200, 10));
    assert_eq!(book.match_order(bid(2, 100, 10)), OrderResult::Added(2));
    assert!(book.trades().is_empty());
    assert_eq!(book.bid_depth(), 1);
    assert_eq!(book.ask_depth(), 1);

    // exact full fill — both sides consumed, trade recorded, maker metadata gone
    let mut book = OrderBook::new();
    book.add_order(ask(1, 100, 10));
    assert_eq!(book.match_order(bid(2, 100, 10)), OrderResult::FullFill(2));
    assert_eq!(book.ask_depth(), 0);
    assert_eq!(book.bid_depth(), 0);
    let t = book.trades();
    assert_eq!(t.len(), 1);
    assert_eq!((t[0].price, t[0].quantity, t[0].maker_id, t[0].taker_id), (100, 10, 1, 2));
    assert_eq!(book.cancel_order(1), OrderResult::NotFound(1)); // metadata purged

    // ask side: selling into bids
    let mut book = OrderBook::new();
    book.add_order(bid(1, 100, 10));
    assert_eq!(book.match_order(ask(2, 100, 10)), OrderResult::FullFill(2));
    assert_eq!(book.bid_depth(), 0);

    // taker smaller than maker: maker stays, level remains
    let mut book = OrderBook::new();
    book.add_order(ask(1, 100, 20));
    assert_eq!(book.match_order(bid(2, 100, 10)), OrderResult::FullFill(2));
    assert_eq!(book.ask_depth(), 1);
    assert_eq!(book.best_ask(), Some(100));
    assert_eq!(book.trades()[0].quantity, 10);

    // taker larger than maker: remainder added as resting order
    let mut book = OrderBook::new();
    book.add_order(ask(1, 100, 5));
    assert_eq!(book.match_order(bid(2, 100, 10)), OrderResult::Added(2));
    assert_eq!(book.ask_depth(), 0);
    assert_eq!(book.bid_depth(), 1);
    assert_eq!(book.trades()[0].quantity, 5);

    // FIFO within a level: first maker fully filled, second partially filled
    let mut book = OrderBook::new();
    book.add_order(ask(1, 100, 5));
    book.add_order(ask(2, 100, 5));
    book.match_order(bid(3, 100, 7));
    let t = book.trades();
    assert_eq!(t.len(), 2);
    assert_eq!((t[0].maker_id, t[0].quantity), (1, 5));
    assert_eq!((t[1].maker_id, t[1].quantity), (2, 2));
    assert_eq!(book.cancel_order(1), OrderResult::NotFound(1)); // fully filled
    assert_eq!(book.cancel_order(2), OrderResult::Cancelled(2)); // still resting

    // multi-level sweep: bid clears first level entirely, takes from second
    let mut book = OrderBook::new();
    book.add_order(ask(1, 100, 10));
    book.add_order(ask(2, 101, 20));
    assert_eq!(book.match_order(bid(3, 105, 15)), OrderResult::FullFill(3));
    assert_eq!(book.ask_depth(), 1);
    assert_eq!(book.best_ask(), Some(101));
    let t = book.trades();
    assert_eq!(t.len(), 2);
    assert_eq!((t[0].price, t[0].quantity), (100, 10));
    assert_eq!((t[1].price, t[1].quantity), (101, 5));

    // full sweep of both levels
    let mut book = OrderBook::new();
    book.add_order(ask(1, 100, 10));
    book.add_order(ask(2, 101, 10));
    assert_eq!(book.match_order(bid(3, 105, 20)), OrderResult::FullFill(3));
    assert_eq!(book.ask_depth(), 0);
    assert_eq!(book.trades().len(), 2);
}
