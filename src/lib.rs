use std::sync::atomic::AtomicU64;
use serde::{Deserialize, Deserializer};
use strum::{EnumCount, EnumIter};

pub mod binance;
pub mod bitstamp;
pub mod exchange_listener;
pub mod server;

#[derive(PartialEq,Eq, Debug, Clone, Copy, EnumCount, EnumIter)]
pub enum Exchange {
    Binance,
    Bitstamp,
}

static ORDER_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct Order {
    pub price: f64,
    pub quantity: f64,
    id: u64,
    pub exchange: Exchange,
}

impl Order {
    pub fn better(&self, other: &Order, bids: bool) -> bool {
        // println!("better ?{:?} {:?}", self, other);
        if bids {
            if self.price > other.price {
                return true;
            } else if self.price < other.price {
                return false;
            }
        } else {
            if self.price < other.price {
                return true;
            } else if self.price > other.price {
                return false;
            }
        }

        if self.quantity > other.quantity {
            return true;
        } else if self.quantity < other.quantity {
            return false;
        }

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
            exchange:Exchange::Binance,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub exchange: Exchange,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self { exchange: Exchange::Binance,
            bids: Default::default(), 
            asks: Default::default() }
    }
}

impl From<bitstamp::api::OrderBook> for OrderBook {
    fn from(book: bitstamp::api::OrderBook) -> Self {
        Self {
            exchange: Exchange::Bitstamp,
            bids: book.bids[0..10].into_iter().map(|el| el.clone().into()).collect(),
            asks: book.asks[0..10].into_iter().map(|el| el.clone().into()).collect(),
        }
    }
}

impl From<binance::api::OrderBook> for OrderBook {
    fn from(book: binance::api::OrderBook) -> Self {
        Self {
            exchange: Exchange::Binance,
            bids: book.bids[0..10].into_iter().map(|el| el.clone().into()).collect(),
            asks: book.asks[0..10].into_iter().map(|el| el.clone().into()).collect(),
        }
    }
}

fn de_float<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}