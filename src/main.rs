use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use futures_util::{SinkExt, StreamExt};


const ENDPOINT: &str = "wss://ws-feed.exchange.coinbase.com";


#[tokio::main]
async fn main() {
    let url = url::Url::parse(ENDPOINT).unwrap();
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (mut write, mut read) = ws_stream.split();

    tokio::spawn(async move {
        let mut cnt = 0;
        loop {
            let msg = read.next().await;
            cnt += 1;
            if let Some(m) = msg {
                if let Ok(ok) =  m {
                    if let Ok(txt ) = ok.to_text() {
                        if txt.len() < 200 {
                            println!("[{}]Rcv: {:?}", cnt, txt);
                        }
                    }
                }
                
            }
        }
    });

    let msg = r#"{
        "type": "subscribe",
        "product_ids": [
            "ETH-USD"
        ],
        "channels": [
            "level2"
        ]
    }"#;

    let _msg2 = r#"{
        "type": "subscribe",
        "product_ids": [
            "ETH-USD",
        ],
        "channels": [
            "level2",
            "heartbeat",
            {
                "name": "ticker",
                "product_ids": [
                    "ETH-BTC",
                    "ETH-USD"
                ]
            }
        ]
    }"#;

    if let Err(e) = write.send(msg.into()).await {
        println!("Send error: {}", e);
        return;
    }
    println!("Msg sent");

    tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;

    println!("End");
}
