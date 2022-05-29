#![allow(non_snake_case)]

use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Order {
    #[serde(deserialize_with = "crate::de_float")]
    pub price: f64,
    #[serde(deserialize_with = "crate::de_float")]
    pub quantity: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrderBook {
    pub lastUpdateId: u64,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
}

impl OrderBook {
    pub fn changed(&self, other: &Self) -> bool {
        self.bids != other.bids || self.asks != other.asks
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SymbolInfo {
    pub symbol: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InfoResponse {
    pub symbols: Vec<SymbolInfo>,
}
