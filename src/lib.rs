#[cfg(feature = "reqwest-lib")]
pub use reqwest::blocking::Client as ReqwestClient;

pub type JsonObject = serde_json::Map<String, serde_json::Value>;

pub trait RequestClient {
    fn get(&self, url: &str) -> JsonObject;

    fn new() -> Self;
}

#[cfg(feature = "ureq-lib")]
pub struct UreqClient;

#[cfg(feature = "ureq-lib")]
impl RequestClient for UreqClient {
    fn get(&self, url: &str) -> JsonObject {
        let response = ureq::get(url).call();
        serde_json::from_reader(response.into_reader()).unwrap()
    }

    fn new() -> Self {
        Self
    }
}

#[cfg(feature = "harp-lib")]
pub struct HarpClient;

#[cfg(feature = "harp-lib")]
impl RequestClient for HarpClient {
    fn get(&self, url: &str) -> JsonObject {
        unimplemented!("harp does not currently support HTTPS");

        use harp::*;

        dbg!(url);

        let (consumed, protocol) = match () {
            _ if url.starts_with("https://") => ("https://".len(), "https"),
            _ if url.starts_with("http://") => ("http://".len(), "http"),
            _ => todo!("implement nice error handling for unknown protocols"),
        };

        let url = &url[consumed..];
        dbg!(url);
        let host_port_separator = url.find(':');
        let host_path_separator = url.find('/');
        let (consumed, host, port) = match (host_port_separator, host_path_separator) {
            (Some(host_port_separator), Some(host_path_separator)) => {
                let mut consumed = host_port_separator;
                let host = &url[..host_port_separator];
                consumed += ":".len();

                let port = &url[consumed..host_path_separator];
                let port = if let Ok(port) = port.parse() {
                    port
                } else {
                    todo!("implement nice error handling for invalid ports")
                };
                consumed += "/".len();

                dbg!(consumed, host, port)
            }
            (None, Some(host_path_separator)) => {
                let mut consumed = host_path_separator;
                let host = &url[..host_path_separator];
                consumed += ":".len();

                // port is not given, determine it from the protocol
                let port = match protocol {
                    "https" => 443,
                    "http" => 80,
                    _ => todo!("implement nice error handling for unknown protocols"),
                };

                dbg!(consumed, host, port)
            }
            (Some(host_port_separator), None) => {
                todo!("host part missing but port given, is this even a valid url?");
            }
            (None, None) => {
                todo!("implement nice error handling for invalid urls");
            }
        };

        let options = Default::default();
        let connection = Connection::open(host, port, &options).unwrap();

        dbg!(&url[consumed - 1..]);
        let path_and_query = &url[consumed..];
        dbg!(path_and_query);

        let body = &request::Body::new(&[]);
        let response = connection.get(path_and_query, body).unwrap().into_vec();
        serde_json::from_slice(&response).unwrap()
    }

    fn new() -> Self {
        Self
    }
}

#[cfg(feature = "reqwest-lib")]
impl RequestClient for ReqwestClient {
    fn get(&self, url: &str) -> JsonObject {
        self.get(url)
            .send()
            .expect("Couldn't send download request")
            .json()
            .expect("Couldn't parse json response")
    }

    fn new() -> Self {
        Self::new()
    }
}

pub struct MockClient;

impl RequestClient for MockClient {
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
where
    T: RequestClient,
{
    pub fn new(apikey: &'a str, client: T) -> Self {
        Self { apikey, client }
    }

    pub fn from_apikey(apikey: &'a str) -> Self {
        let client = T::new();
        Self { apikey, client }
    }
}

include!(concat!(env!("OUT_DIR"), "/gen.rs"));
