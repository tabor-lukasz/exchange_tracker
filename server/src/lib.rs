use serde::{Deserialize, Deserializer};
use std::sync::atomic::AtomicU64;
use strum::{EnumCount, EnumIter};

pub mod binance;
pub mod bitstamp;
pub mod config;
pub mod exchange_listener;
pub mod server;

#[derive(Debug, Clone)]
pub enum TrackerError {
    Cnnection(String),
    Config(String),
    Other(String),
}

impl From<String> for TrackerError {
    fn from(s: String) -> Self {
        TrackerError::Other(s)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, EnumCount, EnumIter)]
pub enum Exchange {
    Binance,
    Bitstamp,
}

/// Counter to sort by creation order in case of eqality in price and amount
static ORDER_ID: AtomicU64 = AtomicU64::new(0);

/// Generalized order data
#[derive(Debug, Clone)]
pub struct Order {
    pub price: f64,
    pub quantity: f64,
    id: u64,
    pub exchange: Exchange,
}

impl Order {
    pub fn better(&self, other: &Order, bids: bool) -> bool {
        // Compare prices
        if bids {
            if self.price > other.price {
                return true;
            } else if self.price < other.price {
                return false;
            }
        } else if self.price < other.price {
            return true;
        } else if self.price > other.price {
            return false;
        }

        // If prices are equal compare quantities
        if self.quantity > other.quantity {
            return true;
        } else if self.quantity < other.quantity {
            return false;
        }

        // If quantities are equal assumbe better = older (smaller id)
        self.id < other.id
    }
}

impl From<bitstamp::api::Order> for Order {
    fn from(order: bitstamp::api::Order) -> Self {
        Self {
            price: order.price,
            quantity: order.quantity,
            id: ORDER_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            exchange: Exchange::Bitstamp,
        }
    }
}

impl From<binance::api::Order> for Order {
    fn from(order: binance::api::Order) -> Self {
        Self {
            price: order.price,
            quantity: order.quantity,
            id: ORDER_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            exchange: Exchange::Binance,
        }
    }
}

/// Generalized order book data
#[derive(Debug, Clone)]
pub struct OrderBook {
    pub exchange: Exchange,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self {
            exchange: Exchange::Binance,
            bids: Default::default(),
            asks: Default::default(),
        }
    }
}

impl From<bitstamp::api::OrderBook> for OrderBook {
    fn from(book: bitstamp::api::OrderBook) -> Self {
        let max_len_bids = std::cmp::min(crate::exchange_listener::MAX_DEPTH, book.bids.len());
        let max_len_asks = std::cmp::min(crate::exchange_listener::MAX_DEPTH, book.asks.len());
        Self {
            exchange: Exchange::Bitstamp,
            bids: book.bids[0..max_len_bids]
                .iter()
                .map(|el| el.clone().into())
                .collect(),
            asks: book.asks[0..max_len_asks]
                .iter()
                .map(|el| el.clone().into())
                .collect(),
        }
    }
}

impl From<binance::api::OrderBook> for OrderBook {
    fn from(book: binance::api::OrderBook) -> Self {
        let max_len_bids = std::cmp::min(crate::exchange_listener::MAX_DEPTH, book.bids.len());
        let max_len_asks = std::cmp::min(crate::exchange_listener::MAX_DEPTH, book.asks.len());
        Self {
            exchange: Exchange::Binance,
            bids: book.bids[0..max_len_bids]
                .iter()
                .map(|el| el.clone().into())
                .collect(),
            asks: book.asks[0..max_len_asks]
                .iter()
                .map(|el| el.clone().into())
                .collect(),
        }
    }
}

/// String -> float deserialize helper for serde
fn de_float<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}
