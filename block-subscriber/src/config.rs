use configuration::error as config_error;

use serde::Deserialize;
use std::time::Duration;
use structopt::StructOpt;

#[derive(StructOpt, Clone, Debug)]
#[structopt(name = "bs_config", about = "Configuration for block subscriber")]
pub struct BSEnvCLIConfig {
    /// Path to block subscriber .toml config
    #[structopt(long, env)]
    pub bs_config: Option<String>,
    /// Max delay (secs) between retries
    #[structopt(long, env)]
    pub bs_max_delay: Option<u64>,
    /// Max retries for block subscriber
    #[structopt(long, env)]
    pub bs_max_retries: Option<usize>,
    /// Timeout value (secs) for block subscriber
    #[structopt(long, env)]
    pub bs_timeout: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct BSFileConfig {
    pub max_delay: Option<u64>,
    pub max_retries: Option<usize>,
    pub timeout: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct FileConfig {
    pub block_subscriber: BSFileConfig,
}

#[derive(Clone, Debug)]
pub struct BSConfig {
    pub max_delay: Duration,
    pub max_retries: usize,
    pub subscriber_timeout: Duration,
}

// default values
const DEFAULT_MAX_DELAY: u64 = 1;
const DEFAULT_MAX_RETRIES: usize = 5;
const DEFAULT_TIMEOUT: u64 = 5;

impl BSConfig {
    pub fn initialize(
        env_cli_config: BSEnvCLIConfig,
    ) -> config_error::Result<Self> {
        let file_config: FileConfig =
            configuration::config::load_config_file(env_cli_config.bs_config)?;

        let max_delay = Duration::from_secs(
            env_cli_config
                .bs_max_delay
                .or(file_config.block_subscriber.max_delay)
                .unwrap_or(DEFAULT_MAX_DELAY),
        );

        let max_retries = env_cli_config
            .bs_max_retries
            .or(file_config.block_subscriber.max_retries)
            .unwrap_or(DEFAULT_MAX_RETRIES);

        let subscriber_timeout = Duration::from_secs(
            env_cli_config
                .bs_timeout
                .or(file_config.block_subscriber.timeout)
                .unwrap_or(DEFAULT_TIMEOUT),
        );

        Ok(BSConfig {
            max_delay,
            max_retries,
            subscriber_timeout,
        })
    }
}
