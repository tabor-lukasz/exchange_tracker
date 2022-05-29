use strum::EnumCount;
use tokio::sync::{mpsc, watch};

use crate::server::Summary;
use crate::{Exchange, TrackerError};

/// Maximum asks and bids size in Summary data
pub const MAX_DEPTH: usize = 10;

pub struct ExchangeListener {
    rx: mpsc::UnboundedReceiver<crate::OrderBook>,
    tx: watch::Sender<Summary>,
    books: Vec<crate::OrderBook>,
}

impl ExchangeListener {
    pub fn new(rx: mpsc::UnboundedReceiver<crate::OrderBook>, tx: watch::Sender<Summary>) -> Self {
        Self {
            rx,
            tx,
            books: vec![crate::OrderBook::default(); Exchange::COUNT],
        }
    }

    pub async fn run(&mut self) -> Result<(), TrackerError> {
        println!("Listener: running");
        let mut last_summary = None;
        loop {
            if let Some(book) = self.rx.recv().await {
                println!("Listener: Received update from {:?}", book.exchange);
                let idx = book.exchange as usize;
                self.books[idx] = book;

                match Self::merge(&self.books) {
                    Ok(v) => {
                        let send = if let Some(last) = &last_summary {
                            // Broadcast only if changed
                            v != *last
                        } else {
                            true
                        };

                        if send {
                            if let Err(_e) = self.tx.send(v.clone()) {
                                // No more receivers - app is shutting down
                                break;
                            }
                            last_summary = Some(v);
                        }
                    }
                    Err(e) => {
                        // Spread calculation error - either bids or asks vec is empty
                        // Not sending update
                        eprintln!("{}", e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Merges partial order books into summary
    fn merge(books: &Vec<crate::OrderBook>) -> Result<Summary, String> {
        let mut bids = Vec::with_capacity(MAX_DEPTH);
        let mut asks = Vec::with_capacity(MAX_DEPTH);

        let mut iters = vec![0usize; books.len()];
        while bids.len() < MAX_DEPTH {
            let mut best = None;
            for i in 0..books.len() {
                if let Some(order) = books[i].bids.get(iters[i]) {
                    if let Some(b) = best {
                        let book: &crate::OrderBook = &books[b];
                        if order.better(&book.bids[iters[b]], true) {
                            best = Some(i);
                        }
                    } else {
                        best = Some(i);
                    }
                }
            }
            if let Some(idx) = best {
                let level = crate::server::Level {
                    exchange: format!("{:?}", books[idx].bids[iters[idx]].exchange),
                    price: books[idx].bids[iters[idx]].price,
                    amount: books[idx].bids[iters[idx]].quantity,
                };
                bids.push(level);
                iters[idx] += 1;
            } else {
                // Not enought bids in source
                break;
            }
        }

        let mut iters = vec![0usize; books.len()];
        while asks.len() < MAX_DEPTH {
            let mut best = None;
            for i in 0..books.len() {
                if let Some(order) = books[i].asks.get(iters[i]) {
                    if let Some(b) = best {
                        let book: &crate::OrderBook = &books[b];
                        if order.better(&book.asks[iters[b]], false) {
                            best = Some(i);
                        }
                    } else {
                        best = Some(i);
                    }
                }
            }
            if let Some(idx) = best {
                let level = crate::server::Level {
                    exchange: format!("{:?}", books[idx].asks[iters[idx]].exchange),
                    price: books[idx].asks[iters[idx]].price,
                    amount: books[idx].asks[iters[idx]].quantity,
                };
                asks.push(level);
                iters[idx] += 1;
            } else {
                // Not enought asks in source
                break;
            }
        }

        let spread = if !asks.is_empty() && !bids.is_empty() {
            asks[0].price - bids[0].price
        } else {
            return Err("Spread undefined".into());
        };

        Ok(Summary { asks, bids, spread })
    }
}

#[cfg(test)]
mod tests {
    use crate::{exchange_listener::ExchangeListener, Exchange, Order, OrderBook};

    #[test]
    fn test_megre() {
        let book1 = OrderBook {
            exchange: Exchange::Bitstamp,
            bids: vec![],
            asks: vec![],
        };

        let book2 = OrderBook {
            exchange: Exchange::Binance,
            bids: vec![],
            asks: vec![],
        };

        let mut books = vec![book1, book2];

        assert!(ExchangeListener::merge(&books).is_err());

        books[0].asks.push(Order {
            price: 10.1,
            quantity: 1.0,
            id: 1,
            exchange: Exchange::Bitstamp,
        });

        books[0].asks.push(Order {
            price: 11.1,
            quantity: 2.0,
            id: 2,
            exchange: Exchange::Bitstamp,
        });

        books[0].asks.push(Order {
            price: 12.1,
            quantity: 3.0,
            id: 3,
            exchange: Exchange::Bitstamp,
        });

        books[0].bids.push(Order {
            price: 9.1,
            quantity: 1.2,
            id: 4,
            exchange: Exchange::Bitstamp,
        });

        books[0].bids.push(Order {
            price: 8.1,
            quantity: 77.0,
            id: 5,
            exchange: Exchange::Bitstamp,
        });

        books[0].bids.push(Order {
            price: 7.1,
            quantity: 3.0,
            id: 6,
            exchange: Exchange::Bitstamp,
        });

        books[1].asks.push(Order {
            price: 10.2,
            quantity: 1.0,
            id: 7,
            exchange: Exchange::Binance,
        });

        books[1].asks.push(Order {
            price: 11.0,
            quantity: 2.0,
            id: 8,
            exchange: Exchange::Binance,
        });

        books[1].asks.push(Order {
            price: 12.1,
            quantity: 5.0,
            id: 9,
            exchange: Exchange::Binance,
        });

        books[1].bids.push(Order {
            price: 9.2,
            quantity: 1.0,
            id: 10,
            exchange: Exchange::Binance,
        });

        books[1].bids.push(Order {
            price: 8.1,
            quantity: 1.0,
            id: 11,
            exchange: Exchange::Binance,
        });

        books[1].bids.push(Order {
            price: 0.1,
            quantity: 3.0,
            id: 12,
            exchange: Exchange::Binance,
        });

        let merged = ExchangeListener::merge(&books).unwrap();

        assert!(merged.spread - 0.9 < f64::EPSILON * 10.0);
        assert!(merged.bids[0].exchange == "Binance");
        assert!(merged.bids[0].amount - 1.0 < f64::EPSILON * 10.0);
        assert!(merged.bids[1].exchange == "Bitstamp");
        assert!(merged.bids[1].amount - 1.2 < f64::EPSILON * 10.0);
        assert!(merged.bids[2].exchange == "Bitstamp");
        assert!(merged.bids[3].amount - 77.0 < f64::EPSILON * 10.0);
    }
}
