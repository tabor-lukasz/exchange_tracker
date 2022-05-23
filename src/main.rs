use crate::coinbase::CoinbaseSubscriber;

mod coinbase;

#[tokio::main]
async fn main() {

    let (tx,_rx) = tokio::sync::mpsc::channel(100);

    let mut coinbase = CoinbaseSubscriber::new(tx).unwrap();

    tokio::spawn(async move {
        coinbase.run().await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    println!("End");
}
