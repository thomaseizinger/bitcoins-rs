//! Pluggable standardized Bitcoin backend

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(unused_extern_crates)]

/// Bitcoin Provider trait
pub mod provider;

/// Pending Transaction
pub mod pending;

/// Outpoint spend watcher
pub mod watcher;

/// Chain watcher
pub mod chain;

/// Utils
pub mod utils;

#[cfg(feature = "esplora")]
/// EsploraProvider
pub mod esplora;

pub use provider::*;

/// The default poll interval, set to 300 seconds (5 minutes)
pub const DEFAULT_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(180 * 1000);

// Alias the default encoder
type Encoder = rmn_btc::Encoder;

// Useful alias for the stateful streams
type ProviderFut<'a, T, P> = std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<T, <P as BTCProvider>::Error>> + 'a + Send>,
>;
