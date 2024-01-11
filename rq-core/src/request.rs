extern crate reqwest;

use once_cell::sync::Lazy;
pub use reqwest::StatusCode;
use reqwest::{header::HeaderMap, Client};

use crate::parser::HttpRequest;
use std::{collections::HashMap, time::Duration};

use self::mime::Payload;

mod decode;
pub mod mime;

static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .no_gzip()
        .build()
        .unwrap()
});

#[derive(Clone)]
pub struct Response {
    pub status: StatusCode,
    pub version: String,
    pub headers: HeaderMap,
    pub payload: Payload,
}

impl Response {
    async fn from_reqwest(value: reqwest::Response) -> Self {
        let status = value.status();
        let version = format!("{:?}", value.version());
        let headers = value.headers().clone();
        let payload = Payload::of_response(value).await;

        Self {
            status,
            version,
            headers,
            payload,
        }
    }
}

type RequestResult = Result<Response, Box<dyn std::error::Error + Send + Sync>>;

pub async fn execute(req: &HttpRequest, params: &HashMap<String, String>) -> RequestResult {
    let request = CLIENT
        .request(req.method.clone(), req.url.fill(params)?)
        .query(&req.query.fill(params)?)
        .headers((&req.headers.fill(params)?).try_into().unwrap())
        .body(req.body.fill(params)?);

    let response = request.send().await?;

    Ok(Response::from_reqwest(response).await)
}
