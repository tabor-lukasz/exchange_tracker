fn main() {
    tonic_build::configure()
        .build_server(true)
        .compile(&["../common/proto/orderbook.proto"], &["../common/proto/"])
        .expect("Failed to build direct messages proto/gRPC definition");
}
