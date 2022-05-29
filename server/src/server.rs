use tokio::sync::watch;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Response;

use crate::server::orderbook_aggregator_server::OrderbookAggregator;

tonic::include_proto!("orderbook");

pub struct OrderbookServer {
    rx: watch::Receiver<Summary>,
}

impl OrderbookServer {
    pub fn new(rx: watch::Receiver<Summary>) -> Self {
        Self { rx }
    }
}

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookServer {
    type BookSummaryStream = ReceiverStream<Result<Summary, tonic::Status>>;

    async fn book_summary(
        &self,
        _request: tonic::Request<Empty>,
    ) -> Result<tonic::Response<Self::BookSummaryStream>, tonic::Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let mut watch_rx = self.rx.clone();
        tokio::spawn(async move {
            loop {
                if watch_rx.changed().await.is_ok() {
                    let summary = watch_rx.borrow().clone();
                    if let Err(_e) = tx.send(Ok(summary)).await {
                        // Client disconnected
                        break;
                    }
                } else {
                    // Listener has dropped app is shutting down
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
