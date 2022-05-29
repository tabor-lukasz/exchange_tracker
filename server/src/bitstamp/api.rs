use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
pub struct SubscribeRequest {
    event: String,
    data: SubscribeData,
}

#[derive(Serialize, Debug)]
pub struct SubscribeData {
    channel: String,
}

impl SubscribeRequest {
    pub fn new(symbol: &str) -> Self {
        Self {
            event: "bts:subscribe".into(),
            data: SubscribeData {
                channel: format!("order_book_{}", symbol),
            },
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct TraidingPairInfo {
    pub url_symbol: String,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Order {
    #[serde(deserialize_with = "crate::de_float")]
    pub price: f64,
    #[serde(deserialize_with = "crate::de_float")]
    pub quantity: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrderBook {
    pub timestamp: String,
    pub microtimestamp: String,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UpdateMsg {
    pub event: String,
    #[allow(dead_code)]
    pub channel: String,
    pub data: Option<OrderBook>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SubsMsg {
    pub event: String,
    #[allow(dead_code)]
    pub channel: String,
}

impl OrderBook {
    pub fn changed(&self, other: &Self) -> bool {
        self.bids != other.bids || self.asks != other.asks
    }
}
