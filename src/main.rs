use alphavantage::*;

fn main() {
    #[cfg(not(feature = "default"))]
    use MockClient as UreqClient;

    let result = AlphavantageClient::new("demo", UreqClient).search_endpoint("test");
    dbg!(result);
}
