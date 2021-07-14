use crate::error::*;

use offchain_core::ethers::core::types::Address;

use im::HashMap;
use serde::{de::DeserializeOwned, Deserialize};
use snafu::ResultExt;
use std::fs;
use structopt::StructOpt;

// TODO: add more options, review the default values
#[derive(StructOpt, Clone, Debug)]
struct EnvCLIConfig {
    /// Path to dispatcher-v2 config
    #[structopt(short, long, env)]
    pub config: Option<String>,
    /// Path to deployment file
    #[structopt(short, long, env)]
    pub deployment: Option<String>,
    /// Provider http endpoint
    #[structopt(short, long, env)]
    pub url: Option<String>,
    /// Provider websocket endpoint
    #[structopt(long, env)]
    pub ws_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct FileConfig {
    pub deployment: Option<String>,
    pub url: Option<String>,
    pub ws_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Config {
    // Holding addresses of all deployed contracts
    pub contracts: HashMap<String, Address>,
    pub url: String,
    pub ws_url: Option<String>,
}

pub fn load_config_file<T: Default + DeserializeOwned>(
    // path to the config file if provided
    config_file: Option<String>,
    // name of the module loading the config
    config_module: &str,
) -> Result<T> {
    match config_file {
        Some(config) => {
            let s = fs::read_to_string(config).map_err(|e| {
                FileError {
                    err: format!(
                        "Fail to read {} config file: {}",
                        config_module, e
                    ),
                }
                .build()
            })?;

            let file_config: T = serde_yaml::from_str(&s).map_err(|e| {
                ParseError {
                    err: format!(
                        "Fail to parse {} config file: {}",
                        config_module, e
                    ),
                }
                .build()
            })?;

            Ok(file_config)
        }
        None => Ok(T::default()),
    }
}

impl Config {
    pub fn initialize() -> Result<Self> {
        let env_cli_config = EnvCLIConfig::from_args();

        let file_config: FileConfig =
            load_config_file(env_cli_config.config, "dispatcher-v2")?;

        let deployment = env_cli_config
            .deployment
            .or(file_config.deployment)
            .unwrap_or(String::from("localhost.json"));

        let contracts = {
            let s = fs::read_to_string(deployment).map_err(|e| {
                FileError {
                    err: format!("Fail to read deployment file: {}", e),
                }
                .build()
            })?;

            let json: serde_json::Value =
                serde_json::from_str(&s).map_err(|e| {
                    ParseError {
                        err: format!("Fail to parse deployment file: {}", e),
                    }
                    .build()
                })?;

            let contracts_object = json["contracts"]
                .as_object()
                .ok_or(snafu::NoneError)
                .context(ParseError {
                    err: "Fail to obtain contracts object from deployment file",
                })?;

            contracts_object.into_iter().try_fold(
                HashMap::new(),
                |contracts, contract| {
                    let address_str = serde_json::to_string(
                        &contract.1["address"],
                    )
                    .map_err(|e| {
                        ParseError {
                            err: format!(
                                "Fail to read address for contract {}: {}",
                                contract.0, e
                            ),
                        }
                        .build()
                    })?;

                    let address: Address = {
                        // Remove leading quotes and '0x', and remove trailing
                        // quotes
                        let leading_offset = 3;
                        let len = address_str.len() - 1;

                        let address_str = &address_str[leading_offset..len];

                        address_str.parse().map_err(|e| {
                            ParseError {
                                err: format!(
                                    "Fail to parse address for contract {}: {}",
                                    contract.0, e
                                ),
                            }
                            .build()
                        })?
                    };

                    Ok(contracts.update(contract.0.clone(), address))
                },
            )?
        };

        let url = env_cli_config
            .url
            .or(file_config.url)
            .unwrap_or(String::from("http://localhost:8545"));

        let ws_url = env_cli_config.ws_url.or(file_config.ws_url);

        Ok(Config {
            contracts,
            url,
            ws_url,
        })
    }
}
