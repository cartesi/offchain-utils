use async_trait::async_trait;
use offchain_core::ethers::middleware::SignerMiddleware;
use offchain_core::ethers::providers::{self, Http, Middleware, Provider, Ws};
use offchain_core::ethers::signers::LocalWallet;
use snafu::{ResultExt, Snafu};
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::sync::Mutex;

///
/// Middleware Factory
#[async_trait]
pub trait MiddlewareFactory {
    ///
    /// User Implementation

    /// Middleware that this factory creates.
    type Middleware: Middleware;

    /// The next MiddlewareFactory in the chain of factories.
    type InnerFactory: MiddlewareFactory<Middleware = <Self::Middleware as Middleware>::Inner>
        + Send
        + Sync;

    /// Get current middleware.
    async fn current(&self) -> Arc<Self::Middleware>;

    /// Get inner factory.
    async fn inner_factory(&self) -> &Self::InnerFactory;

    /// Builds this factory's middleware from inner middleware, and set it as
    /// current.
    async fn build_and_set_middleware(
        &self,
        inner_middleware: Arc<
            <Self::InnerFactory as MiddlewareFactory>::Middleware,
        >,
    ) -> Arc<Self::Middleware>;

    /// Returns if this error should trigger a retry.
    fn should_retry(err: &<Self::Middleware as Middleware>::Error) -> bool;

    ///
    /// Automatic Implementation

    /// Automatic implementation of `new_middleware`. This function receives a
    /// optional middleware. If it is `None`, it will return the current
    /// internal middleware. Otherwise, it will compare the given middleware
    /// with the current internal middleware. If they are different, it will
    /// return the current internal middleware. Otherwise, it will create a new
    /// one, calling on the chain of factories to build a middleware. In all
    /// cases, the returned middleware is always different than the given one.
    async fn new_middleware(
        &self,
        previous: Option<&Self::Middleware>,
    ) -> Result<Arc<Self::Middleware>> {
        let current = self.current().await;

        if let Some(previous) = previous {
            if std::ptr::eq(current.as_ref(), previous) {
                // Get inner middleware and inner factory
                let current_inner = current.inner();
                let inner_factory = self.inner_factory().await;

                // Recursively call `new_middleware` on inner factory.
                let new_inner =
                    inner_factory.new_middleware(Some(current_inner)).await?;

                // Now that we have the new inner middleware, we create out own,
                // setting the current to it.
                let new_middleware =
                    self.build_and_set_middleware(new_inner).await;

                return Ok(new_middleware);
            }
        }

        Ok(current)
    }
}

///
/// "Root" Websocket Middleware Factory
pub struct WsProviderFactory {
    provider: Mutex<Arc<Provider<Ws>>>,
    url: String,
    max_retries: usize,
    max_delay: std::time::Duration,
}

impl WsProviderFactory {
    pub async fn new(
        url: String,
        max_retries: usize,
        max_delay: std::time::Duration,
    ) -> Result<Arc<Self>> {
        let provider =
            WsProviderFactory::new_web3_ws(&url, max_retries, max_delay)
                .await?;

        Ok(Arc::new(Self {
            provider: Mutex::new(Arc::new(provider)),
            url,
            max_retries,
            max_delay,
        }))
    }

    async fn new_web3_ws(
        url: &str,
        max_retries: usize,
        max_delay: std::time::Duration,
    ) -> Result<Provider<Ws>> {
        let mut backoff = backoff::Backoff::new(max_retries, max_delay);
        loop {
            let p_res = Provider::connect(url)
                .await
                .map_err(providers::ProviderError::from)
                .context(ProviderError);

            match p_res {
                Ok(p) => break Ok(p),
                Err(e) => {
                    if backoff.wait().await.is_err() {
                        break RetryLimitReached {
                            retries: max_retries,
                            last_error: Box::new(e),
                        }
                        .fail();
                    }
                }
            }
        }
    }
}

#[async_trait]
impl MiddlewareFactory for WsProviderFactory {
    type Middleware = Provider<Ws>;
    type InnerFactory = Self;

    /// User implemented methods
    async fn current(&self) -> Arc<Self::Middleware> {
        unreachable!("WsProviderFactory `current` unreachable")
    }

    async fn inner_factory(&self) -> &Self::InnerFactory {
        unreachable!("WsProviderFactory `inner_factory` unreachable")
    }

    async fn build_and_set_middleware(
        &self,
        _: Arc<<Self::InnerFactory as MiddlewareFactory>::Middleware>,
    ) -> Arc<Self::Middleware> {
        unreachable!("WsProviderFactory `build_and_set_middleware` unreachable")
    }

    fn should_retry(err: &<Self::Middleware as Middleware>::Error) -> bool {
        // TODO: Improve this retry policy. We may need to change `ethers`
        // to expose inner error types.
        matches!(err, providers::ProviderError::JsonRpcClientError(_))
    }

    /// Default method
    async fn new_middleware(
        &self,
        previous: Option<&Self::Middleware>,
    ) -> Result<Arc<Self::Middleware>> {
        let mut current = self.provider.lock().await;

        if let Some(previous) = previous {
            if std::ptr::eq(current.as_ref(), previous) {
                let new_provider = Arc::new(
                    WsProviderFactory::new_web3_ws(
                        &self.url,
                        self.max_retries,
                        self.max_delay,
                    )
                    .await?,
                );
                *current = Arc::clone(&new_provider);

                return Ok(new_provider);
            }
        }

        Ok(Arc::clone(&current))
    }
}

///
/// "Root" Http Middleware Factory
pub struct HttpProviderFactory {
    provider: Mutex<Arc<Provider<Http>>>,
    url: String,
}

impl HttpProviderFactory {
    pub fn new(url: String) -> Result<Arc<Self>> {
        let provider =
            Provider::<Http>::try_from(url.clone()).context(ParseError {})?;

        Ok(Arc::new(Self {
            provider: Mutex::new(Arc::new(provider)),
            url,
        }))
    }
}

#[async_trait]
impl MiddlewareFactory for HttpProviderFactory {
    type Middleware = Provider<Http>;
    type InnerFactory = Self;

    /// User implemented methods
    async fn current(&self) -> Arc<Self::Middleware> {
        unreachable!("HttpProviderFactory `current` unreachable")
    }

    async fn inner_factory(&self) -> &Self::InnerFactory {
        unreachable!("HttpProviderFactory `inner_factory` unreachable")
    }

    async fn build_and_set_middleware(
        &self,
        _: Arc<<Self::InnerFactory as MiddlewareFactory>::Middleware>,
    ) -> Arc<Self::Middleware> {
        unreachable!(
            "HttpProviderFactory `build_and_set_middleware` unreachable"
        )
    }

    fn should_retry(err: &<Self::Middleware as Middleware>::Error) -> bool {
        // TODO: Improve this retry policy. We may need to change `ethers`
        // to expose inner error types.
        matches!(err, providers::ProviderError::JsonRpcClientError(_))
    }

    /// Default method
    async fn new_middleware(
        &self,
        previous: Option<&Self::Middleware>,
    ) -> Result<Arc<Self::Middleware>> {
        let mut current = self.provider.lock().await;

        if let Some(previous) = previous {
            if std::ptr::eq(current.as_ref(), previous) {
                let new_provider = Arc::new(
                    Provider::<Http>::try_from(self.url.clone())
                        .context(ParseError {})?,
                );
                *current = Arc::clone(&new_provider);

                return Ok(new_provider);
            }
        }

        Ok(Arc::clone(&current))
    }
}

///
/// "Root" Local Signer Middleware Factory
pub struct LocalSignerFactory<IF: MiddlewareFactory> {
    signer: Mutex<Arc<SignerMiddleware<Arc<IF::Middleware>, LocalWallet>>>,
    inner_factory: Arc<IF>,
    wallet: LocalWallet,
}

impl<IF: MiddlewareFactory + Sync + Send> LocalSignerFactory<IF> {
    pub async fn new(
        inner_factory: Arc<IF>,
        wallet: LocalWallet,
    ) -> Result<Arc<Self>> {
        let provider = inner_factory.new_middleware(None).await?;

        Ok(Arc::new(Self {
            signer: Mutex::new(Arc::new(SignerMiddleware::new(
                provider,
                wallet.clone(),
            ))),
            inner_factory,
            wallet,
        }))
    }
}

#[async_trait]
impl<IF: MiddlewareFactory + Sync + Send> MiddlewareFactory
    for LocalSignerFactory<IF>
{
    type Middleware = SignerMiddleware<Arc<IF::Middleware>, LocalWallet>;
    type InnerFactory = IF;

    /// User implemented methods
    async fn current(&self) -> Arc<Self::Middleware> {
        let x = self.signer.lock().await.clone();
        x
    }

    async fn inner_factory(&self) -> &Self::InnerFactory {
        &self.inner_factory
    }

    async fn build_and_set_middleware(
        &self,
        _: Arc<<Self::InnerFactory as MiddlewareFactory>::Middleware>,
    ) -> Arc<Self::Middleware> {
        unreachable!(
            "LocalSignerFactory `build_and_set_middleware` unreachable"
        )
    }

    fn should_retry(_err: &<Self::Middleware as Middleware>::Error) -> bool {
        unreachable!("LocalSignerFactory `should_retry` unreachable")
    }

    // async fn new_middleware(
    //     &self,
    //     previous: Option<&Self::Middleware>,
    // ) -> Result<Arc<Self::Middleware>> {
    //     let mut current = self.signer.lock().await;

    //     if let Some(previous) = previous {
    //         if std::ptr::eq(current.as_ref(), previous) {
    //             let new_provider = self
    //                 .inner_factory
    //                 .new_middleware(Some(previous.inner()))
    //                 .await?;
    //             let new_signer = Arc::new(SignerMiddleware::new(
    //                 new_provider,
    //                 self.wallet.clone(),
    //             ));
    //             *current = Arc::clone(&new_signer);

    //             return Ok(new_signer);
    //         }
    //     }

    //     Ok(Arc::clone(&current))
    // }
}

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    #[snafu(display("Parse error: {}", source))]
    ParseError { source: url::ParseError },

    #[snafu(display("Provider error: {}", source))]
    ProviderError { source: providers::ProviderError },

    #[snafu(display(
        "Retry limit of {} reached, last error: {}",
        retries,
        last_error
    ))]
    RetryLimitReached {
        retries: usize,
        last_error: Box<Error>,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use offchain_core::ethers::providers::{FromErr, Middleware};
    use snafu::Snafu;
    use std::sync::Arc;

    #[derive(Debug)]
    pub struct IdMiddleware<M: Middleware> {
        inner: Arc<M>,
    }

    #[derive(Debug, Snafu)]
    pub enum IdError<M: Middleware + 'static> {
        Inner { source: M::Error },
    }

    impl<M: Middleware> FromErr<M::Error> for IdError<M> {
        fn from(src: M::Error) -> IdError<M> {
            IdError::Inner { source: src }
        }
    }

    #[async_trait]
    impl<M: Middleware + 'static> Middleware for IdMiddleware<M> {
        type Error = IdError<M>; // TODO: `M::Error` doesn't work.
        type Provider = M::Provider;
        type Inner = M;

        fn inner(&self) -> &Self::Inner {
            &self.inner
        }
    }

    pub struct IdFactory<IF: MiddlewareFactory> {
        current:
            Mutex<Arc<IdMiddleware<<IF as MiddlewareFactory>::Middleware>>>,
        inner_factory: Arc<IF>,
    }

    impl<IF: MiddlewareFactory + Send + Sync> IdFactory<IF> {
        async fn new(inner_factory: Arc<IF>) -> Result<Self> {
            let inner_middleware = inner_factory.new_middleware(None).await?;
            let current = Mutex::new(Arc::new(IdMiddleware {
                inner: inner_middleware,
            }));

            Ok(Self {
                current,
                inner_factory,
            })
        }
    }

    #[async_trait]
    impl<IF> MiddlewareFactory for IdFactory<IF>
    where
        IF: MiddlewareFactory + Send + Sync + 'static,
    {
        type Middleware = IdMiddleware<IF::Middleware>;
        type InnerFactory = IF;

        /// User implemented methods, only to be called internally.
        async fn current(&self) -> Arc<Self::Middleware> {
            self.current.lock().await.clone()
        }

        async fn inner_factory(&self) -> &Self::InnerFactory {
            &self.inner_factory
        }

        async fn build_and_set_middleware(
            &self,
            inner_middleware: Arc<
                <Self::InnerFactory as MiddlewareFactory>::Middleware,
            >,
        ) -> Arc<Self::Middleware> {
            let new = Arc::new(IdMiddleware {
                inner: inner_middleware,
            });

            *self.current.lock().await = Arc::clone(&new);
            new
        }

        fn should_retry(_: &<Self::Middleware as Middleware>::Error) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn id_middleware_test() {
        let root_factory =
            HttpProviderFactory::new("http://localhost:8545".to_string())
                .unwrap();
        let id_factory =
            IdFactory::new(Arc::clone(&root_factory)).await.unwrap();

        let m = id_factory.new_middleware(None).await.unwrap();
        let m_same = id_factory.new_middleware(None).await.unwrap();
        assert!(Arc::ptr_eq(&m, &m_same));

        let m2 = id_factory.new_middleware(Some(&m)).await.unwrap();
        assert!(!Arc::ptr_eq(&m, &m2));
    }

    #[tokio::test]
    async fn signer_middleware_test() {
        let root_factory =
            HttpProviderFactory::new("http://localhost:8545".to_string())
                .unwrap();
        let wallet: LocalWallet =
            "380eb0f3d505f087e438eca80bc4df9a7faa24f868e69fc0440261a0fc0567dc"
                .parse()
                .unwrap();
        let signer_factory =
            LocalSignerFactory::new(root_factory, wallet).await.unwrap();

        let m = signer_factory.new_middleware(None).await.unwrap();
        let m_same = signer_factory.new_middleware(None).await.unwrap();
        assert!(Arc::ptr_eq(&m, &m_same));

        let m2 = signer_factory.new_middleware(Some(&m)).await.unwrap();
        assert!(!Arc::ptr_eq(&m, &m2));
    }
}
