use std::str::FromStr;

use clap::{Command, Arg};
use tonic::transport::{Channel, Uri};
use client::orderbook_aggregator_client::OrderbookAggregatorClient;

#[tokio::main]
async fn main() {
    let version = env!("CARGO_PKG_VERSION");

    let matches = Command::new("Exchange tracker")
    .version(version)
    .arg(
        Arg::new("server_addr")
            .help("Server address")
            .short('a')
            .takes_value(true)
            .required(true),
    )
    .get_matches();

    let addr_str = matches.value_of("server_addr").unwrap();

    let mut client = OrderbookAggregatorClient::new(
        Channel::builder(Uri::from_str(&format!("https://{}",addr_str)
    ).expect("Uri parse error")).connect().await.expect("Failed to connect"));

    let req = tonic::Request::new(client::Empty{});
    let mut stream = client.book_summary(req).await.expect("Failed to get stream").into_inner();

    loop  {
        match stream.message().await {
            Ok(msg) => {
                if let Some (m) = msg {
                    println!("Summary:\nSpread: {}\nBids: {:?}\nAsks: {:?}", m.spread, m.bids, m.asks);
                } else {
                    println!("Stream ended");
                    break;
                }
            }, 
            Err(e) => {
                eprintln!("Streamming error: {}", e);
                break;
            }

        }
    }
}
