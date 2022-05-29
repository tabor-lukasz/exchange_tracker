use futures_util::{
    stream::{SplitSink, SplitStream},
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, WebSocketStream,
};

pub mod api;

type WsSink =
    SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>;
type WsStream =
    SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>;

// const ENDPOINT: &str = "wss://stream.binance.com:9443";

const PARTIAL_DEPTH_ENDPOINT: &str = "wss://ws.bitstamp.net";

enum ConnectionStatus {
    Disconnected,
    Connected,
    SubscribtionSent,
    Subscribed,
    Updating,
}

pub struct BitstampSubscriber {
    status: ConnectionStatus,
    tx: mpsc::UnboundedSender<crate::OrderBook>,
    url: url::Url,
    ws: Option<(WsSink, WsStream)>,
    last_book: Option<api::OrderBook>,
}

impl BitstampSubscriber {
    pub fn new(tx: mpsc::UnboundedSender<crate::OrderBook>) -> Result<Self, String> {
        Ok(Self {
            status: ConnectionStatus::Disconnected,
            tx,
            url: url::Url::parse(PARTIAL_DEPTH_ENDPOINT).map_err(|e| e.to_string())?,
            ws: None,
            last_book: None,
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
        println!("Bitstamp: Connecting");
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| format!("Ws Connection error {}", e))?;
        println!("Bitstamp: WebSocket handshake has been successfully completed");
        self.ws = Some(ws_stream.split());

        Ok(())
    }

    async fn subscribe_to_channel(&mut self) -> Result<(), String> {
        let req = api::SubscribeRequest::new("ethbtc".into());
        let serialized = serde_json::to_string(&req).expect("Valid json");
        self.ws.as_mut().expect("Is connected").0
            .send(serialized.into())
            .await
            .map_err(|e| format!("Subscribe send error: {}", e))?;
        println!("1");
        Ok(())
    }

    async fn rcv_subscription_info(&mut self) -> Result<(), String> {
        let msg = self.ws.as_mut().expect("Is connected").1
            .next()
            .await
            .ok_or(format!("Ws stream terminated"))?
            .map_err(|e| format!("Rcv error {}", e))?
            .into_text()
            .map_err(|_e| format!("Recieved msg is not a string"))?;

        println!("{}", msg);
        // panic!();

        // let _ : SubscripctionInfo = serde_json::from_str(&msg).map_err(|e| format!("Subscription info parse error: {}",e))?;
        
        Ok(())
    }

    async fn rcv_snapshot(&mut self) -> Result<(), String> {
        // let text = reqwest::get(SNAPSHOT_ENDPOINT)
        //     .await.unwrap()
        //     .text()
        //     .await.unwrap();

        // let snapshot : OrderBookSnapshot = serde_json::from_str(&text).map_err(|e| format!("Snapshot parse error: {}",e))?;

        // println!("{} {}", snapshot.asks.len(), snapshot.bids.len());
        // println!("{}", text);
        
        Ok(())
    }

    async fn rcv_update(&mut self) -> Result<(), String> {
        let msg = self.ws.as_mut().expect("Is connected").1
            .next()
            .await
            .ok_or(format!("Ws stream terminated"))?
            .map_err(|e| format!("Rcv error {}", e))?
            .into_text()
            .map_err(|_e| format!("Recieved msg is not a string"))?;

        // println!("{}", msg);

        let event: api::EventMsg = serde_json::from_str(&msg).map_err(|e| format!("Bitstamp Update info parse error: {}",e))?;

        let book = if let Some(book) = event.data {
            book
        } else {
            return Err("Invalid event sequence".into());
        };

        if let Some(u) = &self.last_book {
            if book.changed(u) {
                // println!("Bitstamp");
                // println!("asks {:?}", &book.asks[0..3]);
                // println!("bids {:?}", &book.bids[0..3]);
                self.tx.send(book.clone().into()).unwrap();
                self.last_book = Some(book);
            }
        } else {
            // println!("Bitstamp");
            // println!("asks {:?}", &book.asks[0..3]);
            // println!("bids {:?}", &book.bids[0..3]);
            self.tx.send(book.clone().into()).unwrap();
            self.last_book = Some(book);
        }
        
        Ok(())
    }
}
