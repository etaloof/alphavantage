use alphavantage::*;

fn main() {
    AlphavantageClient::new("demo", MockClient)
        .search_endpoint("test");
}