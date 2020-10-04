# Rust client for the Alpha Vantage API

A Rust wrapper for the [Alpha Vantage API](https://www.alphavantage.co/). It can be used to access the data from four categories:

- Time Series Stock data
- Fundamental Data (different financial data points)
- Physical and Digital/Crypto Currencies
- Technical Indicators

For more information, visit the official [Alpha Vantage API Documentation](https://www.alphavantage.co/documentation). Note: According to the [FAQ section](https://www.alphavantage.co/support) every wrapper/library has to provide the original response of the Alpha Vantage API to library users. The methods of this wrapper returns a `Result<JsonObject, AlphavantageError>`. The `JsonObject` is just a `serde_json::Map<String, serde_json::Value>` and includes the original response as a JSON object. In case of an network or parsing error, an `AlphavantageError` is returned. 

## Usage
You can choose an HTTP client depending on your needs. Currently, [reqwest]() and [ureq](https://crates.io/crates/ureq) are supported. `ureq` is the default. To use reqwest, disable the default features (see the [cargo reference](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#choosing-features)), and enable the `reqwest-lib` feature. These HTTP clients are publicly exported by this crate so you don't need to add them to your dependencies. 

If for some reason another HTTP client is better suited for your purposes you can implement the `RequestClient` trait for this HTTP client (e. g. using a [new type](https://doc.rust-lang.org/stable/rust-by-example/generics/new_types.html)).

You can then construct an `AlphavantageClient` with `AlphavantageClient::new(apikey, client)` where the `apikey` is your [Alpha Vantage API key](https://www.alphavantage.co/support/) and `client` is an instance of your chosen HTTP client. The `AlphavantageClient` implementations a trait for each of the API categories as listed above and has a method for each of the request endpoints. The below example uses the ureq HTTP client to query the Alpha Vantage API for stock symbols matching the string `"test"` (using the endpoint documented at [here](https://www.alphavantage.co/documentation/#symbolsearch)).

```rust
use alphavantage::*;

fn main() {
    let result = AlphavantageClient::new("demo", UreqClient).search_endpoint("test");
    dbg!(result);
}
```
