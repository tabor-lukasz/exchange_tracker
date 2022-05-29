use exchange_tracker::{binance::BinanceSubscriber, bitstamp::BitstampSubscriber, exchange_listener::{ExchangeListener, MergedOrderBook}, server::Summary};

#[tokio::main]
async fn main() {

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let (merged_tx, mut merged_rx) = tokio::sync::watch::channel(Summary::default());

    let mut binance = BinanceSubscriber::new(tx.clone()).unwrap();
    let mut bitstamp = BitstampSubscriber::new(tx).unwrap();
    let mut listener = ExchangeListener::new(rx,merged_tx);

    tokio::spawn(async move {
        listener.run().await;
    });

    tokio::spawn(async move {
        binance.run().await;
    });

    tokio::spawn(async move {
        bitstamp.run().await;
    });

    tokio::spawn(async move {
        loop {
            if merged_rx.changed().await.is_ok() {
                let merged = merged_rx.borrow();
                print!("updated asks {}: ", merged.asks.len());
                for b in &merged.asks {
                    print!("{} {} {:?} ", b.price, b.amount, b.exchange);
                }
                print!("\nupdated bids {}: ", merged.bids.len());
                for b in &merged.bids {
                    print!("{} {} {:?} ", b.price, b.amount, b.exchange);
                }
                println!();
            } else {
                // Listener has dropped
                break;
            }
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(15000)).await;



    println!("End");
}
