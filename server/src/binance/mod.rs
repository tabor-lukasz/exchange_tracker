use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};

use crate::{config::BinanceConfig, TrackerError};

use self::api::InfoResponse;

pub mod api;

type WsSink =
    SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>;
type WsStream =
    SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>;

const INFO_ENDPOINT: &str = "https://api.binance.com/api/v3/exchangeInfo";
const DEPTH_ENDPOINT_PREFIX: &str = "wss://stream.binance.com:9443/ws/";
const DEPTH_ENDPOINT_SUFFIX: &str = "@depth10@100ms";
const EX_NAME: &str = "Binance";

enum ConnectionStatus {
    Disconnected,
    Updating,
}

pub struct BinanceSubscriber {
    cfg: BinanceConfig,
    status: ConnectionStatus,
    tx: mpsc::UnboundedSender<crate::OrderBook>,
    ws: Option<(WsSink, WsStream)>,
    last_book: Option<api::OrderBook>,
}

impl BinanceSubscriber {
    pub fn new(
        cfg: BinanceConfig,
        tx: mpsc::UnboundedSender<crate::OrderBook>,
    ) -> Result<Self, String> {
        Ok(Self {
            cfg,
            status: ConnectionStatus::Disconnected,
            tx,
            ws: None,
            last_book: None,
        })
    }

    pub async fn run(&mut self) -> Result<(), TrackerError> {
        loop {
            if let Err(e) = self.process().await {
                if let TrackerError::Cnnection(_e) = &e {
                    eprintln!("{}: {:?}", EX_NAME, e);
                    self.status = ConnectionStatus::Disconnected;
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    println!("{}: Retrying.", EX_NAME);
                } else {
                    return Err(e);
                }
            }
        }
    }

    async fn process(&mut self) -> Result<(), TrackerError> {
        match &mut self.status {
            ConnectionStatus::Disconnected => {
                self.check_config().await?;
                self.connect().await?;
                self.status = ConnectionStatus::Updating;
            }
            ConnectionStatus::Updating => {
                self.rcv_update().await?;
            }
        }
        Ok(())
    }

    async fn connect(&mut self) -> Result<(), TrackerError> {
        println!("{}: Connecting", EX_NAME);
        let (ws_stream, _) = connect_async(&format!(
            "{}{}{}",
            DEPTH_ENDPOINT_PREFIX,
            self.cfg.symbol.to_lowercase(),
            DEPTH_ENDPOINT_SUFFIX
        ))
        .await
        .map_err(|e| TrackerError::Cnnection(format!("Ws Connection error {}", e)))?;

        println!(
            "{}: WebSocket handshake has been successfully completed",
            EX_NAME
        );

        self.ws = Some(ws_stream.split());

        Ok(())
    }

    async fn check_config(&self) -> Result<(), TrackerError> {
        let resp = reqwest::get(INFO_ENDPOINT)
            .await
            .map_err(|e| TrackerError::Other(format!("Get info error: {}", e)))?;
        let txt = resp
            .text()
            .await
            .map_err(|e| TrackerError::Other(format!("{}: {}", EX_NAME, e)))?;
        let info: InfoResponse = serde_json::from_str(&txt)
            .map_err(|e| TrackerError::Other(format!("Info response parse error: {}", e)))?;
        let symbols: Vec<String> = info.symbols.into_iter().map(|s| s.symbol).collect();
        if !symbols.contains(&self.cfg.symbol) {
            return Err(TrackerError::Config(format!(
                "{}: Invalid symbol {} Valid symbols are:\n{:?}",
                EX_NAME, self.cfg.symbol, symbols
            )));
        }

        Ok(())
    }

    async fn rcv_update(&mut self) -> Result<(), TrackerError> {
        let msg = self
            .ws
            .as_mut()
            .expect("Is connected")
            .1
            .next()
            .await
            .ok_or_else( || TrackerError::Cnnection(format!(
                "{}: Ws stream terminated",
                EX_NAME
            )))?
            .map_err(|e| TrackerError::Cnnection(format!("{}: Rcv error {}", EX_NAME, e)))?;

        if msg.is_ping() {
            println!("{}: Ping", EX_NAME);
            let data = msg.into_data();
            self.ws
                .as_mut()
                .expect("Is connected")
                .0
                .send(Message::Pong(data))
                .await
                .map_err(|e| TrackerError::Cnnection(format!("{}: Send error {}", EX_NAME, e)))?;

            println!("{}: Pong", EX_NAME);

            return Ok(());
        }

        let text = msg
            .into_text()
            .map_err(|e| TrackerError::Other(format!("{}: Msg to text error {}", EX_NAME, e)))?;

        let book: api::OrderBook = serde_json::from_str(&text).map_err(|e| {
            TrackerError::Other(format!(
                "{}: Update msg parse error: {}\n{}",
                EX_NAME, e, text
            ))
        })?;

        if let Some(u) = &self.last_book {
            if book.changed(u) {
                self.tx.send(book.clone().into()).map_err(|e| {
                    TrackerError::Other(format!("{}: Book send error: {}", EX_NAME, e))
                })?;
                self.last_book = Some(book);
            }
        } else {
            self.tx
                .send(book.clone().into())
                .map_err(|e| TrackerError::Other(format!("{}: Book send error: {}", EX_NAME, e)))?;
            self.last_book = Some(book);
        }

        Ok(())
    }
}
