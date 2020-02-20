pub use reqwest::blocking::Client as ReqwestClient;

pub type JsonObject = serde_json::Map<String, serde_json::Value>;

pub trait MockableClient {
    fn get(&self, url: &str) -> JsonObject;

    fn new() -> Self;
}

impl MockableClient for ReqwestClient {
    fn get(&self, url: &str) -> JsonObject {
        self.get(url)
            .send().expect("Couldn't send download request")
            .json().expect("Couldn't parse json response")
    }

    fn new() -> Self {
        Self::new()
    }
}

pub struct MockClient;

impl MockableClient for MockClient {
    fn get(&self, url: &str) -> JsonObject {
        eprintln!("MockClient: Making request to {}", url);
        serde_json::Map::new()
    }

    fn new() -> Self {
        Self
    }
}

pub struct AlphavantageClient<'a, T> {
    apikey: &'a str,
    client: T,
}

impl<'a, T> AlphavantageClient<'a, T>
    where T: MockableClient {
    pub fn new(apikey: &'a str, client: T) -> Self {
        Self { apikey, client }
    }

    pub fn from_apikey(apikey: &'a str) -> Self {
        let client = T::new();
        Self { apikey, client }
    }
}

include!(concat!(env!("OUT_DIR"), "/gen.rs"));