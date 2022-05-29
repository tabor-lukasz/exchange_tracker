use std::path::Path;

use clap::{Arg, Command};
use exchange_tracker::{
    binance::BinanceSubscriber,
    bitstamp::BitstampSubscriber,
    exchange_listener::ExchangeListener,
    server::{orderbook_aggregator_server::OrderbookAggregatorServer, OrderbookServer, Summary},
};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() {
    let version = env!("CARGO_PKG_VERSION");

    let matches = Command::new("Exchange tracker")
        .version(version)
        .arg(
            Arg::new("config_path")
                .help("Path to the YAML configuration file")
                .short('c')
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let path_str = matches.value_of("config_path").unwrap();
    let path = Path::new(path_str);
    if !path.exists() {
        panic!(
            "Provided configuration file path does not exist: {}",
            path_str
        );
    }

    let str = std::fs::read_to_string(path).expect("Failed to read configuration file");
    let config: exchange_tracker::config::ServerConfig =
        serde_yaml::from_str(str.as_str()).expect("Failed to deserialize configuration file");

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let (merged_tx, merged_rx) = tokio::sync::watch::channel(Summary::default());

    let mut binance = BinanceSubscriber::new(config.binance.clone(), tx.clone()).unwrap();
    let mut bitstamp = BitstampSubscriber::new(config.bitstamp.clone(), tx).unwrap();
    let mut listener = ExchangeListener::new(rx, merged_tx);

    let (_tx, shutdown_rx_handle) = oneshot::channel::<()>();
    let grpc_srv = OrderbookAggregatorServer::new(OrderbookServer::new(merged_rx));

    let grpc_future = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(grpc_srv)
            .serve_with_shutdown(
                config.grpc_listen_addr.parse().expect("Invalid grpc url"),
                async move {
                    let _ = shutdown_rx_handle.await;
                },
            )
            .await
            .expect("Failed to start grpc server");
    });

    tokio::select! {
        r = grpc_future => {
            if let Err(r) = r {
                println!("{:?}", r);
            }
        },
        r = listener.run() => {
            if let Err(r) = r {
                println!("{:?}", r);
            }
        },
        r = binance.run() => {
            if let Err(r) = r {
                println!("{:?}", r);
            }
        },
        r = bitstamp.run() => {
            if let Err(r) = r {
                println!("{:?}", r);
            }
        }
    }

    println!("End");
}
