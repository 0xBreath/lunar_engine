use hex::encode as hex_encode;
use hmac::{Hmac, Mac};
use reqwest::StatusCode;
use reqwest::blocking::Response;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT, CONTENT_TYPE};
use sha2::Sha256;
use serde::de::{DeserializeOwned, Error};
use crate::api::API;
use crate::errors::{BinanceError, BinanceContentError, Result};

#[derive(Clone)]
pub struct Client {
  api_key: String,
  secret_key: String,
  host: String,
  inner_client: reqwest::blocking::Client,
}

impl Client {
  pub fn new(api_key: Option<String>, secret_key: Option<String>, host: String) -> Self {
    Client {
      api_key: api_key.unwrap_or_default(),
      secret_key: secret_key.unwrap_or_default(),
      host,
      inner_client: reqwest::blocking::Client::builder()
        .pool_idle_timeout(None)
        .build()
        .unwrap(),
    }
  }

  pub fn get_signed<T: DeserializeOwned>(
    &self, endpoint: API, request: Option<String>,
  ) -> Result<T> {
    let url = self.sign_request(endpoint, request);
    let client = &self.inner_client;
    let response = client
      .get(url.as_str())
      .headers(self.build_headers(true)?)
      .send().map_err(|e| {
        BinanceError::Reqwest(e)
      })?;

    self.handler(response)
  }

  pub fn post_signed<T: DeserializeOwned>(&self, endpoint: API, request: String) -> Result<T> {
    let url = self.sign_request(endpoint, Some(request));
    let client = &self.inner_client;
    let response = client
      .post(url.as_str())
      .headers(self.build_headers(true)?)
      .send().map_err(|e| {
        BinanceError::Reqwest(e)
      })?;

    self.handler(response)
  }

  #[allow(dead_code)]
  pub fn delete_signed<T: DeserializeOwned>(
    &self, endpoint: API, request: Option<String>,
  ) -> Result<T> {
    let url = self.sign_request(endpoint, request);
    let client = &self.inner_client;
    let response = client
      .delete(url.as_str())
      .headers(self.build_headers(true)?)
      .send().map_err(|e| {
        BinanceError::Reqwest(e)
      })?;

    self.handler(response)
  }

  pub fn get<T: DeserializeOwned>(&self, endpoint: API, request: Option<String>) -> Result<T> {
    let mut url: String = format!("{}{}", self.host, String::from(endpoint));
    if let Some(request) = request {
      if !request.is_empty() {
        url.push_str(format!("?{}", request).as_str());
      }
    }

    let client = &self.inner_client;
    let response = client.get(url.as_str()).send().map_err(|e| {
      BinanceError::Reqwest(e)
    })?;

    self.handler(response)
  }

  #[allow(dead_code)]
  pub fn post<T: DeserializeOwned>(&self, endpoint: API) -> Result<T> {
    let url: String = format!("{}{}", self.host, String::from(endpoint));

    let client = &self.inner_client;
    let response = client
      .post(url.as_str())
      .headers(self.build_headers(false)?)
      .send().map_err(|e| {
        BinanceError::Reqwest(e)
      })?;

    self.handler(response)
  }

  pub fn put<T: DeserializeOwned>(&self, endpoint: API, listen_key: &str) -> Result<T> {
    let url: String = format!("{}{}", self.host, String::from(endpoint));
    let data: String = format!("listenKey={}", listen_key);

    let client = &self.inner_client;
    let response = client
      .put(url.as_str())
      .headers(self.build_headers(false)?)
      .body(data)
      .send().map_err(|e| {
        BinanceError::Reqwest(e)
      })?;

    self.handler(response)
  }

  #[allow(dead_code)]
  pub fn delete<T: DeserializeOwned>(&self, endpoint: API, listen_key: &str) -> Result<T> {
    let url: String = format!("{}{}", self.host, String::from(endpoint));
    let data: String = format!("listenKey={}", listen_key);

    let client = &self.inner_client;
    let response = client
      .delete(url.as_str())
      .headers(self.build_headers(false)?)
      .body(data)
      .send().map_err(|e| {
        BinanceError::Reqwest(e)
      })?;

    self.handler(response)
  }

  fn sign_request(&self, endpoint: API, request: Option<String>) -> String {
    if let Some(request) = request {
      let mut signed_key =
        Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
      signed_key.update(request.as_bytes());
      let signature = hex_encode(signed_key.finalize().into_bytes());
      let request_body: String = format!("{}&signature={}", request, signature);
      format!("{}{}?{}", self.host, String::from(endpoint), request_body)
    } else {
      let signed_key = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
      let signature = hex_encode(signed_key.finalize().into_bytes());
      let request_body: String = format!("&signature={}", signature);
      format!("{}{}?{}", self.host, String::from(endpoint), request_body)
    }
  }

  fn build_headers(&self, content_type: bool) -> Result<HeaderMap> {
    let mut custom_headers = HeaderMap::new();

    custom_headers.insert(USER_AGENT, HeaderValue::from_static("lunar-engine-test"));
    if content_type {
      custom_headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-www-form-urlencoded"),
      );
    }
    custom_headers.insert(
      HeaderName::from_static("x-mbx-apikey"),
      HeaderValue::from_str(self.api_key.as_str()).expect("invalid api key"),
    );

    Ok(custom_headers)
  }

  fn handler<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
    match response.status() {
      StatusCode::OK => Ok(response.json::<T>().map_err(|e| {
        let serde_error = serde_json::Error::custom(e.to_string());
        BinanceError::Serde(serde_error)
      })?),
      StatusCode::INTERNAL_SERVER_ERROR => {
        Err(BinanceError::Reqwest(response.error_for_status().unwrap_err()))
      }
      StatusCode::SERVICE_UNAVAILABLE => {
        Err(BinanceError::Reqwest(response.error_for_status().unwrap_err()))
      }
      StatusCode::UNAUTHORIZED => {
        let error: BinanceContentError = response.json().map_err(|e| {
          BinanceError::Reqwest(e)
        })?;
        Err(BinanceError::Content(error))
      }
      StatusCode::BAD_REQUEST => {
        let error: BinanceContentError = response.json().map_err(|e| {
          BinanceError::Reqwest(e)
        })?;
        Err(BinanceError::Content(error))
      }
      _s => {
        Err(BinanceError::Other(response.error_for_status().unwrap_err().to_string()))
      }
    }
  }
}