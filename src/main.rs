use alphavantage::*;

fn main() {
    let result = AlphavantageClient::new("demo", UreqClient).search_endpoint("test");
    dbg!(result);
}
