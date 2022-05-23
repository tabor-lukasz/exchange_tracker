use futures_util::{
    stream::{SplitSink, SplitStream},
};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, WebSocketStream,
};

use crate::coinbase::api::{OrderBookSnapshot, SubscripctionInfo, OrderBookUpdate};

use self::api::SubscribeRequest;

pub mod api;

type WsSink =
    SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>;
type WsStream =
    SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>;

const ENDPOINT: &str = "wss://ws-feed.exchange.coinbase.com";

enum ConnectionStatus {
    Disconnected,
    Connected,
    SubscribtionSent,
    Subscribed,
    Updating,
}

pub struct CoinbaseSubscriber {
    status: ConnectionStatus,
    tx: tokio::sync::mpsc::Sender<Vec<f64>>,
    url: url::Url,
    ws: Option<(WsSink, WsStream)>
}

impl CoinbaseSubscriber {
    pub fn new(tx: tokio::sync::mpsc::Sender<Vec<f64>>) -> Result<Self, String> {
        Ok(Self {
            status: ConnectionStatus::Disconnected,
            tx,
            url: url::Url::parse(ENDPOINT).map_err(|e| e.to_string())?,
            ws: None,
        })
    }

    pub async fn run(&mut self) {
        loop {
            if let Err(e) = self.process().await {
                println!("{}", e);
                self.status = ConnectionStatus::Disconnected;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                println!("Retrying.");
            }
        }
    }

    async fn process(&mut self) -> Result<(), String> {
        match &mut self.status {
            ConnectionStatus::Disconnected => {
                self.connect().await?;
                self.status = ConnectionStatus::Connected;
            }
            ConnectionStatus::Connected => {
                self.subscribe_to_channel().await?;
                self.status = ConnectionStatus::SubscribtionSent;
            }
            ConnectionStatus::SubscribtionSent => {
                self.rcv_subscription_info().await?;
                self.status = ConnectionStatus::Subscribed;
            }
            ConnectionStatus::Subscribed => {
                self.rcv_snapshot().await?;
                self.status = ConnectionStatus::Updating;
            }
            ConnectionStatus::Updating => {
                self.rcv_update().await?;
            }
        }
        Ok(())
    }

    async fn connect(&mut self) -> Result<(), String> {
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| format!("Ws Connection error {}", e))?;
        println!("WebSocket handshake has been successfully completed");
        self.ws = Some(ws_stream.split());

        Ok(())
    }

    async fn subscribe_to_channel(&mut self) -> Result<(), String> {
        let req = SubscribeRequest::new("ETH-USD".into());
        let serialized = serde_json::to_string(&req).expect("Valid json");
        self.ws.as_mut().expect("Is connected").0
            .send(serialized.into())
            .await
            .map_err(|e| format!("Subscribe send error: {}", e))?;
        Ok(())
    }

    async fn rcv_subscription_info(&mut self) -> Result<(), String> {
        let msg = self.ws.as_mut().expect("Is connected").1
            .next()
            .await
            .ok_or(format!("Ws stream terminated"))?
            .map_err(|e| format!("Rcv error {}", e))?
            .into_text()
            .map_err(|e| format!("Recieved msg is not a string"))?;

        let _ : SubscripctionInfo = serde_json::from_str(&msg).map_err(|e| format!("Subscription info parse error: {}",e))?;
        
        Ok(())
    }

    async fn rcv_snapshot(&mut self) -> Result<(), String> {
        let msg = self.ws.as_mut().expect("Is connected").1
            .next()
            .await
            .ok_or(format!("Ws stream terminated"))?
            .map_err(|e| format!("Rcv error {}", e))?
            .into_text()
            .map_err(|e| format!("Recieved msg is not a string"))?;

        let snapshot : OrderBookSnapshot = serde_json::from_str(&msg).map_err(|e| format!("Snapshot parse error: {}",e))?;

        println!("{} {}", snapshot.asks.len(), snapshot.bids.len());
        
        Ok(())
    }

    async fn rcv_update(&mut self) -> Result<(), String> {
        let msg = self.ws.as_mut().expect("Is connected").1
            .next()
            .await
            .ok_or(format!("Ws stream terminated"))?
            .map_err(|e| format!("Rcv error {}", e))?
            .into_text()
            .map_err(|e| format!("Recieved msg is not a string"))?;

        let upadte : OrderBookUpdate = serde_json::from_str(&msg).map_err(|e| format!("Update info parse error: {}",e))?;

        for c in upadte.changes {
            println!("{}\t{}", c.price, c.size);
        }
        
        Ok(())
    }
}
