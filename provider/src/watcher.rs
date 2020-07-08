use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures_core::stream::Stream;
use futures_util::stream::StreamExt;
use pin_project::pin_project;

use rmn_btc::prelude::*;

use crate::{provider::BTCProvider, DEFAULT_POLL_INTERVAL, utils::{StreamLast, interval}};

type ProviderFut<'a, T, P> =
    Pin<Box<dyn Future<Output = Result<T, <P as BTCProvider>::Error>> + 'a + Send>>;

enum WatcherStates<'a, P: BTCProvider> {
    // Waiting for a tx to spend
    WaitingSpends(ProviderFut<'a, Option<TXID>, P>),
    // Tx known, getting confs
    WaitingMoreConfs(usize, TXID, ProviderFut<'a, Option<usize>, P>),
    // Future has completed, and should panic if polled again
    Completed,
}

/// Polls the API for the tx that spends an outpoint
///
/// TODO: refactor to use `Outspend`?
#[pin_project]
pub struct PollingWatcher<'a, P: BTCProvider> {
    outpoint: BitcoinOutpoint,
    confirmations: usize,
    state: WatcherStates<'a, P>,
    interval: Box<dyn Stream<Item = ()> + Send + Unpin>,
    provider: &'a P,
}

impl<'a, P: BTCProvider> PollingWatcher<'a, P> {
    /// Creates a new outspend poller
    pub fn new(outpoint: BitcoinOutpoint, provider: &'a P) -> Self {
        let fut = Box::pin(provider.get_outspend(outpoint));
        Self {
            outpoint,
            confirmations: 0,
            state: WatcherStates::WaitingSpends(fut),
            interval: Box::new(interval(DEFAULT_POLL_INTERVAL)),
            provider,
        }
    }

    /// Sets the number of confirmations before being notified of the spend
    pub fn confirmations(mut self, confs: usize) -> Self {
        self.confirmations = confs;
        self
    }

    /// Sets the polling interval
    pub fn interval<T: Into<Duration>>(mut self, duration: T) -> Self {
        self.interval = Box::new(interval(duration.into()));
        self
    }
}

impl<P: BTCProvider> StreamLast for PollingWatcher<'_, P> {}


impl<'a, P: BTCProvider> futures::stream::Stream for PollingWatcher<'a, P> {
    type Item = (usize, TXID);

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.state {
            WatcherStates::WaitingSpends(fut) => {
                let _ready = futures_util::ready!(this.interval.poll_next_unpin(ctx));

                if let Ok(Some(txid)) = futures_util::ready!(fut.as_mut().poll(ctx)) {
                    // If we want >0 confs, go to getting confs
                    let fut = Box::pin(this.provider.get_confs(txid));
                    *this.state = WatcherStates::WaitingMoreConfs(0, txid, fut);
                    return Poll::Ready(Some((0, txid)));
                } else {
                    // Continue otherwise
                    let fut = Box::pin(this.provider.get_outspend(*this.outpoint));
                    *this.state = WatcherStates::WaitingSpends(fut);
                }
            },
            WatcherStates::WaitingMoreConfs(previous_confs, txid, fut) => {
                let _ready = futures_util::ready!(this.interval.poll_next_unpin(ctx));

                if let Ok(Some(confs)) = futures_util::ready!(fut.as_mut().poll(ctx)) {
                    // If we're not at our limit
                    if confs > *previous_confs && confs < *this.confirmations {
                        let t = *txid;
                        let fut = Box::pin(this.provider.get_confs(t));
                        *this.state = WatcherStates::WaitingMoreConfs(confs, t, fut);
                        return Poll::Ready(Some((confs, t)));
                    }

                    // If we have enough confs, ready now
                    if confs >= *this.confirmations {
                        let t = *txid;
                        *this.state = WatcherStates::Completed;
                        ctx.waker().wake_by_ref();
                        return Poll::Ready(Some((confs, t)));
                    }
                }
                // If we want more confs, repeat
                let fut = Box::pin(this.provider.get_confs(*txid));
                *this.state = WatcherStates::WaitingMoreConfs(*previous_confs, *txid, fut);
            }

            WatcherStates::Completed => {
                return Poll::Ready(None);
            }
        };
        Poll::Pending
    }
}
