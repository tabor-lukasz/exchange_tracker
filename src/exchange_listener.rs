use strum::EnumCount;
use tokio::sync::{Mutex, mpsc, watch};

use crate::Exchange;
use crate::server::Summary;

const MAX_DEPTH: usize = 10;


#[derive(Debug, Clone, Default)]
pub struct MergedOrderBook {
    pub asks: Vec<crate::Order>,
    pub bids: Vec<crate::Order>,
}


pub struct ExchangeListener {
    rx: mpsc::UnboundedReceiver<crate::OrderBook>,
    tx: watch::Sender<Summary>,
    books: Mutex<Vec<crate::OrderBook>>,
}

impl ExchangeListener {
    pub fn new(rx: mpsc::UnboundedReceiver<crate::OrderBook>, tx: watch::Sender<Summary>) -> Self {
        
        Self {
            rx,
            tx,
            books: Mutex::new(vec![crate::OrderBook::default(); Exchange::COUNT]),
        }
    }

    pub async fn run(&mut self) {
        println!("Listener running");
        loop {
            if let Some(book) = self.rx.recv().await {
                let mut guard = self.books.lock().await;
                let idx = book.exchange as usize;
                guard[idx] = book;
                match Self::merge(&*guard) {
                    Ok(v) => {
                        if let Err(_e) = self.tx.send(v) {
                            // No more receivers - app is shutting down
                            break;
                        }
                    },
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
                
            }
        }
    }

    fn merge(books: &Vec<crate::OrderBook>) -> Result<Summary, String> {
        let mut bids = Vec::with_capacity(MAX_DEPTH);
        let mut asks = Vec::with_capacity(MAX_DEPTH);

        // println!("merging...");
        // println!("Binance...");
        // let len = std::cmp::min(10,books[0].asks.len());
        // for i in 0..len {
        //     print!("{} ", books[0].asks[i].price);    
        // }
        // println!("");
        // for i in 0..len {
        //     print!("{} ", books[0].bids[i].price);
        // }
        // println!("");
        // println!("Bitstamp...");
        // let len = std::cmp::min(10,books[1].asks.len());
        // for i in 0..len {
        //     print!("{} ", books[1].asks[i].price);    
        // }
        // println!("");
        // for i in 0..len {
        //     print!("{} ", books[1].bids[i].price);
        // }
        // println!();

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
                    exchange: format!("{:?}", books[idx].asks[iters[idx]].exchange),
                    price: books[idx].asks[iters[idx]].price,
                    amount: books[idx].asks[iters[idx]].quantity,
                };
                bids.push(level);
                iters[idx] += 1;
            } else {
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
                break;
            }
        }

        let spread = if !asks.is_empty() && !bids.is_empty() {
            asks[0].price - bids[0].price
        } else {
            return Err("Spread undefined".into());
        };

        Ok(Summary {
            asks,
            bids,
            spread,
        })
    }
}