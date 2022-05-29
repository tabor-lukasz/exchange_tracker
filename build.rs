fn main() {
    tonic_build::configure()
        .build_server(true)
        .compile(
            &[
                "src/proto/orderbook.proto",
            ],
            &["src/proto/"],
        )
        .expect("Failed to build direct messages proto/gRPC definition");
}