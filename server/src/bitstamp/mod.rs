use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};

use crate::config::BitstampConfig;
use crate::TrackerError;

use self::api::TraidingPairInfo;

pub mod api;

type WsSink =
    SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>;
type WsStream =
    SplitStream<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>;

const EX_ENDPOINT: &str = "wss://ws.bitstamp.net";
const EX_NAME: &str = "Bitstamp";
const INFO_ENDPOINT: &str = "https://www.bitstamp.net/api/v2/trading-pairs-info/";

enum ConnectionStatus {
    Disconnected,
    Connected,
    SubscribtionSent,
    Updating,
}

pub struct BitstampSubscriber {
    cfg: BitstampConfig,
    status: ConnectionStatus,
    tx: mpsc::UnboundedSender<crate::OrderBook>,
    ws: Option<(WsSink, WsStream)>,
    last_book: Option<api::OrderBook>,
}

impl BitstampSubscriber {
    pub fn new(
        cfg: BitstampConfig,
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
                self.status = ConnectionStatus::Connected;
            }
            ConnectionStatus::Connected => {
                self.subscribe_to_channel().await?;
                self.status = ConnectionStatus::SubscribtionSent;
            }
            ConnectionStatus::SubscribtionSent => {
                self.rcv_subscription_info().await?;
                self.status = ConnectionStatus::Updating;
            }
            ConnectionStatus::Updating => {
                self.rcv_update().await?;
            }
        }
        Ok(())
    }

    async fn check_config(&self) -> Result<(), TrackerError> {
        let resp = reqwest::get(INFO_ENDPOINT)
            .await
            .map_err(|e| TrackerError::Other(format!("{}: Get info error: {}", EX_NAME, e)))?;

        let txt = resp
            .text()
            .await
            .map_err(|e| TrackerError::Other(format!("{}: {}", EX_NAME, e)))?;

        let info: Vec<TraidingPairInfo> = serde_json::from_str(&txt).map_err(|e| {
            TrackerError::Other(format!("{}: Info response parse error: {}", EX_NAME, e))
        })?;

        let symbols: Vec<String> = info.into_iter().map(|s| s.url_symbol).collect();
        if !symbols.contains(&self.cfg.symbol.to_lowercase()) {
            return Err(TrackerError::Config(format!(
                "{}: Invalid symbol {} Valid symbols are:\n{:?}",
                EX_NAME, self.cfg.symbol, symbols
            )));
        }

        Ok(())
    }

    async fn connect(&mut self) -> Result<(), TrackerError> {
        println!("{}: Connecting", EX_NAME);
        let (ws_stream, _) = connect_async(EX_ENDPOINT)
            .await
            .map_err(|e| format!("Ws Connection error {}", e))?;
        println!(
            "{}: WebSocket handshake has been successfully completed",
            EX_NAME
        );
        self.ws = Some(ws_stream.split());

        Ok(())
    }

    async fn subscribe_to_channel(&mut self) -> Result<(), TrackerError> {
        let req = api::SubscribeRequest::new(&self.cfg.symbol.to_lowercase());
        let serialized = serde_json::to_string(&req).expect("Valid json");
        self.ws
            .as_mut()
            .expect("Is connected")
            .0
            .send(serialized.into())
            .await
            .map_err(|e| {
                TrackerError::Cnnection(format!("{}: Subscribe send error: {}", EX_NAME, e))
            })?;

        Ok(())
    }

    async fn rcv_subscription_info(&mut self) -> Result<(), TrackerError> {
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
            .map_err(|e| TrackerError::Cnnection(format!("Rcv error {}", e)))?
            .into_text()
            .map_err(|_e| format!("{}: Recieved msg is not a string", EX_NAME))?;

        let event: api::SubsMsg = serde_json::from_str(&msg).map_err(|e| {
            format!(
                "{}:  Subscription response parse error: {}\n{}",
                EX_NAME, e, msg
            )
        })?;

        if !event.event.contains("subscription_succeeded") {
            return Err(format!("{}: Invalid subscription response: {:?}", EX_NAME, event).into());
        } else {
            println!("{}: Channel subscribed", EX_NAME);
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
            .ok_or(format!("{}: Ws stream terminated", EX_NAME))?
            .map_err(|e| format!("{}: Rcv error {}", EX_NAME, e))?;

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

        if msg.is_empty() || !msg.is_text() {
            println!("{}: Empty update", EX_NAME);
            return Ok(());
        }

        let text = msg
            .into_text()
            .map_err(|e| TrackerError::Other(format!("{}: Msg to text error {}", EX_NAME, e)))?;

        let event: api::UpdateMsg = serde_json::from_str(&text)
            .map_err(|e| format!("{}: Update info parse error: {}\n{}", EX_NAME, e, text))?;

        let book = if let Some(book) = event.data {
            book
        } else {
            return Err(format!("{}: Invalid event sequence", EX_NAME).into());
        };

        if let Some(u) = &self.last_book {
            if book.changed(u) {
                self.tx.send(book.clone().into()).unwrap();
                self.last_book = Some(book);
            }
        } else {
            self.tx.send(book.clone().into()).unwrap();
            self.last_book = Some(book);
        }

        Ok(())
    }
}
