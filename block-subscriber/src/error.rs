use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error<M: offchain_core::ethers::providers::Middleware + 'static> {
    #[snafu(display("Ethers provider error: {}", source))]
    EthersProviderError { source: M::Error },

    #[snafu(display("Got incomplete block"))]
    BlockIncomplete { err: String },

    #[snafu(display("New block subscriber timeout: {}", source))]
    NewBlockSubscriberTimeout { source: std::io::Error },

    #[snafu(display("Web3 subscription dropped"))]
    SubscriptionDropped {},

    #[snafu(display("Retry limit of {} reached", retries))]
    RetryLimitReached {
        retries: usize,
        last_error: Box<Error<M>>,
    },

    #[snafu(display("Factory error: {}", source))]
    FactoryError { source: middleware_factory::Error },
}

pub type Result<T, M> = std::result::Result<T, Error<M>>;
