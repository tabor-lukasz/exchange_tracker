use futures_util::{
    stream::{SplitSink, SplitStream},
};
use futures_util::{StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, WebSocketStream,
};

use crate::Exchange;

pub mod api;

type WsSink =
    SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>;
type WsStream =
    SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>;

const DEPTH_ENDPOINT: &str = "wss://stream.binance.com:9443/ws/ethbtc@depth10@100ms";

enum ConnectionStatus {
    Disconnected,
    Updating,
}

pub struct BinanceSubscriber {
    ex_type: Exchange,
    status: ConnectionStatus,
    tx: mpsc::UnboundedSender<crate::OrderBook>,
    url: url::Url,
    ws: Option<(WsSink, WsStream)>,
    last_book: Option<api::OrderBook>,
}

impl BinanceSubscriber {
    pub fn new(tx: mpsc::UnboundedSender<crate::OrderBook>) -> Result<Self, String> {
        Ok(Self {
            ex_type: Exchange::Binance,
            status: ConnectionStatus::Disconnected,
            tx,
            url: url::Url::parse(DEPTH_ENDPOINT).map_err(|e| e.to_string())?,
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
                self.status = ConnectionStatus::Updating;
            }
            ConnectionStatus::Updating => {
                self.rcv_update().await?;
            }
        }
        Ok(())
    }

    async fn connect(&mut self) -> Result<(), String> {
        println!("{:?}: Connecting", self.ex_type);
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| format!("Ws Connection error {}", e))?;
        println!("{:?} WebSocket handshake has been successfully completed", self.ex_type);
        self.ws = Some(ws_stream.split());

        Ok(())
    }

    async fn rcv_update(&mut self) -> Result<(), String> {
        let msg = self.ws.as_mut().expect("Is connected").1
            .next()
            .await
            .ok_or(format!("Ws stream terminated"))?
            .map_err(|e| format!("Rcv error {}", e))?
            .into_text()
            .map_err(|e| format!("Msg to test error {}", e))?;

        let book: api::OrderBook = serde_json::from_str(&msg).map_err(|e| format!("Update msg parse error: {}",e))?;

        if let Some(u) = &self.last_book {
            if book.changed(u) {
                println!("Binance");
                println!("asks {:?}", &book.asks[0..3]);
                println!("bids {:?}", &book.bids[0..3]);
                self.tx.send(book.clone().into()).map_err(|e| format!("Book send error: {}", e))?;
                self.last_book = Some(book);
            }
        } else {
            println!("Binance");
            println!("asks {:?}", &book.asks[0..3]);
            println!("bids {:?}", &book.bids[0..3]);
            self.tx.send(book.clone().into()).map_err(|e| format!("Book send error: {}", e))?;
            self.last_book = Some(book);
        }

        Ok(())
    }
}
