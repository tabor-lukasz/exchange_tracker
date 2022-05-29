use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct BinanceConfig {
    pub symbol: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BitstampConfig {
    pub symbol: String,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub grpc_listen_addr: String,
    pub binance: BinanceConfig,
    pub bitstamp: BitstampConfig,
}
