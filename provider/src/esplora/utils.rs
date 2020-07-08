use serde::Deserialize;
use thiserror::Error;

use riemann_core::{hashes::marked::MarkedDigest, ser::ByteFormat};
use rmn_btc::prelude::TXID;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// An error type returned by the networking layer. We alias this to abstract it and unify
/// the SwapError::APIError type.
#[cfg(target_arch = "wasm32")]
pub type RequestError = JsValue;

/// An error type returned by the networking layer. We alias this to abstract it and unify
/// the SwapError::APIError type.
#[cfg(not(target_arch = "wasm32"))]
pub type RequestError = reqwest::Error;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error("RequestError: {0:?}")]
    RequestError(RequestError),
}

impl From<RequestError> for FetchError {
    fn from(v: RequestError) -> FetchError {
        FetchError::RequestError(v)
    }
}

/// Fetch a raw hex transaction by its BE txid
pub(crate) async fn fetch_tx_hex(api_root: &str, txid_be: &str) -> Result<String, FetchError> {
    let url = format!("{}/tx/{}/hex", api_root, txid_be);
    ez_fetch_string(&url).await
}

/// Fetch a raw hex transaction by its TXID
pub(crate) async fn fetch_tx_hex_by_id(api_root: &str, txid: TXID) -> Result<String, FetchError> {
    fetch_tx_hex(api_root, &txid.reversed().serialize_hex().unwrap()).await
}

pub(crate) async fn fetch_it(url: &str) -> Result<reqwest::Response, FetchError> {
    Ok(reqwest::Client::new().get(url).send().await?)
}

/// Easy fetching of a URL. Attempts to serde JSON deserialize the result
pub(crate) async fn ez_fetch_json<T: for<'a> Deserialize<'a>>(url: &str) -> Result<T, FetchError> {
    let res = fetch_it(url).await?;
    let text = res.text().await?;
    Ok(serde_json::from_str(&text)?)
}

/// Easy fetching of a URL. Returns result as a String
pub(crate) async fn ez_fetch_string(url: &str) -> Result<String, FetchError> {
    let res = fetch_it(url).await?;
    let text = res.text().await?;
    Ok(text)
}

pub(crate) async fn post_str(url: &str, body: &str) -> Result<String, FetchError> {
    Ok(reqwest::Client::new()
        .post(url)
        .body(body.to_owned())
        .send()
        .await?
        .text()
        .await?)
}

/// Easy posting hex to a url
pub(crate) async fn post_hex<T>(url: &str, bytes: T) -> Result<String, FetchError>
where
    T: AsRef<[u8]>,
{
    post_str(url, &hex::encode(bytes)).await
}
