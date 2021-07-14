use configuration::error as config_error;

use serde::Deserialize;
use std::time::Duration;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "bs_config", about = "Configuration for block subscriber")]
struct BSEnvCLIConfig {
    /// Path to block subscriber config
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
    pub bs_max_delay: Option<u64>,
    pub bs_max_retries: Option<usize>,
    pub bs_timeout: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct BSConfig {
    pub max_delay: Duration,
    pub max_retries: usize,
    pub subscriber_timeout: Duration,
}

impl BSConfig {
    pub fn initialize() -> config_error::Result<Self> {
        let env_cli_config = BSEnvCLIConfig::from_args();

        let file_config: BSFileConfig =
            configuration::config::load_config_file(
                env_cli_config.bs_config,
                "block subscriber",
            )?;

        let max_delay = Duration::from_secs(
            env_cli_config
                .bs_max_delay
                .or(file_config.bs_max_delay)
                .unwrap_or(1),
        );

        let max_retries = env_cli_config
            .bs_max_retries
            .or(file_config.bs_max_retries)
            .unwrap_or(5);

        let subscriber_timeout = Duration::from_secs(
            env_cli_config
                .bs_timeout
                .or(file_config.bs_timeout)
                .unwrap_or(5),
        );

        Ok(BSConfig {
            max_delay,
            max_retries,
            subscriber_timeout,
        })
    }
}
