use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Debug, Clone)]
pub struct SubscribeRequest {
    pub r#type: String,
    pub product_ids: Vec<String>,
    pub channels: Vec<String>,
}

impl SubscribeRequest {
    pub fn new(product_id: String) -> Self {
        Self {
            r#type: "subscribe".into(),
            product_ids: vec![product_id],
            channels: vec!["level2".into()],
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChannelInfo {
    pub name: String,
    pub product_ids: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SubscripctionInfo {
    pub r#type: String,
    pub channels: Vec<ChannelInfo>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrderBookEntry {
    #[serde(deserialize_with = "de_float")]
    pub price: f64,
    #[serde(deserialize_with = "de_float")]
    pub size: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrderBookSnapshot {
    pub r#type: String,
    pub product_id: String,
    pub bids: Vec<OrderBookEntry>,
    pub asks: Vec<OrderBookEntry>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Change {
    pub product_id: String,
    #[serde(deserialize_with = "de_float")]
    pub price: f64,
    #[serde(deserialize_with = "de_float")]
    pub size: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrderBookUpdate {
    r#type: String,
    product_id: String,
    pub changes: Vec<Change>,
}

fn de_float<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}