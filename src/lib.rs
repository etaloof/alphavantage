use thiserror::Error;

#[derive(Error, Debug)]
pub enum AlphavantageError {
    #[error("could not deserialize into json")]
    InvalidJson(#[from] serde_json::Error),
    #[error("the network request failed")]
    Network(Box<dyn std::error::Error>),
}

pub type JsonObject = serde_json::Map<String, serde_json::Value>;

pub trait RequestClient {
    fn get(&self, url: &str) -> Result<JsonObject, AlphavantageError>;

    fn new() -> Self;
}

#[cfg(feature = "ureq-lib")]
pub struct UreqClient;

#[cfg(feature = "ureq-lib")]
impl RequestClient for UreqClient {
    fn get(&self, url: &str) -> Result<JsonObject, AlphavantageError> {
        let response = ureq::get(url).call();
        if let Some(err) = response.synthetic_error() {
            Err(err.into())
        } else {
            let reader = response.into_reader();
            Ok(serde_json::from_reader(reader)?)
        }
    }

    fn new() -> Self {
        Self
    }
}

#[cfg(feature = "ureq-lib")]
impl From<&ureq::Error> for AlphavantageError {
    fn from(err: &ureq::Error) -> Self {
        AlphavantageError::Network(err.to_string().into())
    }
}

#[cfg(feature = "harp-lib")]
pub struct HarpClient;

#[cfg(feature = "harp-lib")]
impl RequestClient for HarpClient {
    fn get(&self, url: &str) -> Result<JsonObject, AlphavantageError> {
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
        let path_and_query = &url[consumed..];

        let opts = &Default::default();
        let connection = Connection::open(host, port, opts).unwrap();

        let body = &Default::default();
        let response = connection.get(path_and_query, body).unwrap().into_vec();
        let reader = std::io::Cursor::new(response);
        Ok(serde_json::from_reader(reader)?)
    }

    fn new() -> Self {
        Self
    }
}

#[cfg(feature = "reqwest-lib")]
pub use reqwest::blocking::Client as ReqwestClient;

#[cfg(feature = "reqwest-lib")]
impl RequestClient for ReqwestClient {
    fn get(&self, url: &str) -> Result<JsonObject, AlphavantageError> {
        let response = self.get(url)
            .send()?
            .bytes()?;
        let reader = std::io::Cursor::new(response);
        Ok(serde_json::from_reader(reader)?)
    }

    fn new() -> Self {
        Self::new()
    }
}

#[cfg(feature = "reqwest-lib")]
impl From<reqwest::Error> for AlphavantageError {
    fn from(value: reqwest::Error) -> Self {
        AlphavantageError::Network(value.to_string().into())
    }
}

pub struct MockClient;

impl RequestClient for MockClient {
    fn get(&self, url: &str) -> Result<JsonObject, AlphavantageError> {
        eprintln!("MockClient: Making request to {}", url);
        Ok(serde_json::Map::new())
    }

    fn new() -> Self {
        Self
    }
}

impl<F> RequestClient for F
    where
        F: Fn(&str) -> Result<JsonObject, AlphavantageError>,
{
    fn get(&self, url: &str) -> Result<JsonObject, AlphavantageError> {
        self(url)
    }

    fn new() -> Self {
        panic!("AlphavantageClient cannot be constructed from api key when RequestClient is a function")
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
