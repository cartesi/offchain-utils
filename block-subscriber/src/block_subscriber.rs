use crate::error::*;
use middleware_factory::MiddlewareFactory;
use offchain_core::types::Block;

use async_trait::async_trait;
use offchain_core::ethers::providers::{Middleware, PubsubClient};
use snafu::ResultExt;
use std::convert::TryInto;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio_stream::{Stream, StreamExt};

/// NewBlockSubscriber is an object responsible for listening to new block
/// events from the blockchain and broadcasting them to whoever has subscribed.
#[async_trait]
pub trait NewBlockSubscriber {
    async fn subscribe(&self) -> Option<broadcast::Receiver<Block>>;
}

pub struct BlockSubscriberHandle<M: Middleware + 'static> {
    pub handle: tokio::task::JoinHandle<Result<(), M>>,
    pub kill_switch: oneshot::Sender<()>,
}

pub struct BlockSubscriber<MF>
where
    MF: MiddlewareFactory,
    <<MF as MiddlewareFactory>::Middleware as Middleware>::Provider:
        PubsubClient,
{
    factory: Arc<MF>,
    subscriber_timeout: std::time::Duration,
    max_retries: usize,
    max_delay: std::time::Duration,
    channel: Mutex<Option<broadcast::Sender<Block>>>,
}

impl<MF> BlockSubscriber<MF>
where
    MF: MiddlewareFactory + Send + Sync + 'static,
    <<MF as MiddlewareFactory>::Middleware as Middleware>::Provider:
        PubsubClient,
    <<<MF as MiddlewareFactory>::Middleware as Middleware>::Provider as PubsubClient>::NotificationStream:
         Send,
{
    /// Must keep the `Sender` part of the `kill_switch` in scope. Dropping the `Sender`
    /// will cause the `BlockSubscriber` to terminate.
    pub fn create_and_start(
        factory: Arc<MF>,
        subscriber_timeout: std::time::Duration,
        max_retries: usize,
        max_delay: std::time::Duration,
    ) -> (
        Arc<Self>,
        BlockSubscriberHandle<<MF as MiddlewareFactory>::Middleware>,
    ) {
        let (kill_tx, kill_rx) = oneshot::channel();

        let (tx, _) = broadcast::channel(1024);
        let this = Arc::new(BlockSubscriber {
            factory,
            subscriber_timeout,
            max_retries,
            max_delay,
            channel: Mutex::new(Some(tx)),
        });
        let handle = BlockSubscriber::start(Arc::clone(&this), kill_rx);

        (
            this,
            BlockSubscriberHandle {
                handle,
                kill_switch: kill_tx,
            },
        )
    }
}

#[async_trait]
impl<MF> NewBlockSubscriber for BlockSubscriber<MF>
where
    MF: MiddlewareFactory + Send + Sync,
    <<MF as MiddlewareFactory>::Middleware as Middleware>::Provider:
        PubsubClient + Send,
{
    async fn subscribe(&self) -> Option<broadcast::Receiver<Block>> {
        match &*self.channel.lock().await {
            Some(channel) => Some(channel.subscribe()),
            None => None,
        }
    }
}

/// Internals
impl<MF> BlockSubscriber<MF>
where
    MF: MiddlewareFactory + Send + Sync + 'static,
    <<MF as MiddlewareFactory>::Middleware as Middleware>::Provider:
        PubsubClient,
    <<<MF as MiddlewareFactory>::Middleware as Middleware>::Provider as PubsubClient>::NotificationStream:
         Send,
{
    fn start(
        self: Arc<Self>,
        kill_switch: oneshot::Receiver<()>,
    ) -> tokio::task::JoinHandle<
        Result<(), <MF as MiddlewareFactory>::Middleware>,
    > {
        // Create background task and detach it.
        tokio::spawn(async move {
            // Create future future of `background_process` main loop. This
            // future will run against the kill_switch.
            let task = self.background_process();
            tokio::pin!(task);

            tokio::select! {
                res = &mut task => {
                    let mut channel = self.channel.lock().await;
                    *channel = None;
                    return res
                },

                _ = kill_switch => {
                    let mut channel = self.channel.lock().await;
                    *channel = None;
                    return Ok(())
                }
            }
        })
    }

    async fn background_process(
        &self,
    ) -> Result<(), <MF as MiddlewareFactory>::Middleware> {
        let mut middleware = self.new_middleware(None).await?;

        // Loop and retry on error.
        loop {
            middleware = self.new_middleware(Some(&middleware)).await?;

            // Subscribe to new blocks, retrying if it fails.
            let mut backoff =
                backoff::Backoff::new(self.max_retries, self.max_delay);
            let subscription = loop {
                let res = middleware
                    .subscribe_blocks()
                    .await
                    .context(EthersProviderError)
                    .map(|subscription| {
                        Box::pin(
                            subscription.timeout(self.subscriber_timeout).map(
                                |x| {
                                    let block_header = x
                                        .map_err(|e| e.into())
                                        .context(NewBlockSubscriberTimeout)?;

                                    let block = block_header
                                        .try_into()
                                        .map_err(|err| {
                                            BlockIncomplete { err }.build()
                                        })?;

                                    Ok(block)
                                },
                            ),
                        )
                    });

                match res {
                    // Subscription successful, break.
                    Ok(s) => {
                        break s;
                    }

                    // Subscription error. Backoff, reset middleware, loop.
                    Err(e) => {
                        backoff.wait().await.map_err(|()| {
                            RetryLimitReached {
                                retries: self.max_retries,
                                last_error: Box::new(e),
                            }
                            .build()
                        })?;
                    }
                }
            };

            // Main loop. Retry on error.
            let res = self.listen_and_broadcast(subscription).await;
            match res {
                // The channel was dropped, break from loop.
                Ok(()) => return Ok(()),

                Err(_) => {
                    // TODO: warn error.
                }
            }
        }
    }

    async fn listen_and_broadcast(
        &self,
        mut subscription: impl Stream<Item = Result<Block, <MF as MiddlewareFactory>::Middleware>>
            + Send
            + Unpin,
    ) -> Result<(), <MF as MiddlewareFactory>::Middleware> {
        // Listen to new blocks and notify subscribers.
        loop {
            // Block on waiting for new block.
            let new_head = subscription
                .next()
                .await
                .ok_or(snafu::NoneError)
                .context(SubscriptionDropped)??;

            // Send new block to subscribers.
            let res = match &*self.channel.lock().await {
                Some(channel) => channel.send(new_head),
                None => return Ok(()), // Channel dropped by kill_switch,
            };
            if let Err(_) = res {
                // TODO: warn there are no subscribers.
            }
        }
    }

    async fn new_middleware(
        &self,
        previous: Option<&<MF as MiddlewareFactory>::Middleware>,
    ) -> Result<
        <MF as MiddlewareFactory>::Middleware,
        <MF as MiddlewareFactory>::Middleware,
    > {
        self.factory
            .new_middleware(previous)
            .await
            .context(FactoryError)
    }
}
