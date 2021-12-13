use crate::error::*;

use offchain_core::ethers;

use ethers::core::rand::thread_rng;
use ethers::core::types::Address;
use ethers::signers::{coins_bip39::English, LocalWallet, MnemonicBuilder};
use im::HashMap;
use rpassword::prompt_password_stdout;
use serde::{de::DeserializeOwned, Deserialize};
use snafu::ResultExt;
use std::fs;
use std::path::Path;
use structopt::StructOpt;

// TODO: add more options, review the default values
#[derive(StructOpt, Clone, Debug)]
pub struct EnvCLIConfig {
    /// Path to offchain .toml config
    #[structopt(short, long, env)]
    pub config: Option<String>,
    /// Path to deployment file
    #[structopt(short, long, env)]
    pub deployment: Option<String>,
    /// Mnemonic phrase of the wallet
    #[structopt(long, env)]
    pub mnemonic: Option<String>,
    /// Seed of the wallet
    #[structopt(long, env)]
    pub seed: Option<String>,
    /// Provider http endpoint
    #[structopt(short, long, env)]
    pub url: Option<String>,
    /// Provider websocket endpoint
    #[structopt(long, env)]
    pub ws_url: Option<String>,
    /// Create the wallet if file doesn't exist
    #[structopt(long, env)]
    pub wallet_create: Option<bool>,
    /// Path to the password encrypted wallet json file
    #[structopt(long, env)]
    pub wallet_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct OffchainFileConfig {
    pub deployment: Option<String>,
    pub mnemonic: Option<String>,
    pub seed: Option<String>,
    pub url: Option<String>,
    pub ws_url: Option<String>,
    pub wallet_create: Option<bool>,
    pub wallet_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct FileConfig {
    pub offchain: OffchainFileConfig,
}

#[derive(Clone, Debug)]
pub struct Config {
    // Holding addresses of all deployed contracts
    pub contracts: HashMap<String, Address>,
    pub url: String,
    pub ws_url: Option<String>,
    pub wallet: Option<LocalWallet>,
}

// default values
const DEFAULT_URL: &str = "http://localhost:8545";
const DEFAULT_WALLET_CREATE: bool = false;

pub fn load_config_file<T: Default + DeserializeOwned>(
    // path to the config file if provided
    config_file: Option<String>,
) -> Result<T> {
    match config_file {
        Some(config) => {
            let s = fs::read_to_string(&config).map_err(|e| {
                FileError {
                    err: format!(
                        "Fail to read config file {}, error: {}",
                        config, e
                    ),
                }
                .build()
            })?;

            let file_config: T = toml::from_str(&s).map_err(|e| {
                ParseError {
                    err: format!("Fail to parse config {}, error: {}", s, e),
                }
                .build()
            })?;

            Ok(file_config)
        }
        None => Ok(T::default()),
    }
}

impl Config {
    pub fn initialize(env_cli_config: EnvCLIConfig) -> Result<Self> {
        let file_config: FileConfig = load_config_file(env_cli_config.config)?;

        let deployment = env_cli_config
            .deployment
            .or(file_config.offchain.deployment);

        let contracts = match deployment {
            Some(deployment_file) => {
                let s = fs::read_to_string(deployment_file).map_err(|e| {
                    FileError {
                        err: format!("Fail to read deployment file: {}", e),
                    }
                    .build()
                })?;

                let json: serde_json::Value = serde_json::from_str(&s)
                    .map_err(|e| {
                        ParseError {
                            err: format!(
                                "Fail to parse deployment file: {}",
                                e
                            ),
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
                        let address_str =
                            serde_json::to_string(&contract.1["address"])
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
                            // Remove leading quotes and '0x', and remove
                            // trailing quotes
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
            }
            None => HashMap::new(),
        };

        let url = env_cli_config
            .url
            .or(file_config.offchain.url)
            .unwrap_or(String::from(DEFAULT_URL));

        let ws_url = env_cli_config.ws_url.or(file_config.offchain.ws_url);

        let mnemonic =
            env_cli_config.mnemonic.or(file_config.offchain.mnemonic);

        // TODO: support seed
        let _seed = env_cli_config.seed.or(file_config.offchain.seed);

        let wallet_create = env_cli_config
            .wallet_create
            .or(file_config.offchain.wallet_create)
            .unwrap_or(DEFAULT_WALLET_CREATE);

        let wallet_path = env_cli_config
            .wallet_path
            .or(file_config.offchain.wallet_path);

        // Priority of loading a wallet
        // Wallet file path > mnemonic > seed
        let wallet = if let Some(path) = wallet_path {
            let path = Path::new(&path);
            match (path.exists(), wallet_create) {
                (true, _) => {
                    let password = prompt_password_stdout("Enter password: ")
                        .map_err(|e| {
                        ParseError {
                            err: format!(
                                "Fail to parse password for wallet, error: {}",
                                e
                            ),
                        }
                        .build()
                    })?;
                    Some(
                        LocalWallet::decrypt_keystore(&path, &password)
                            .map_err(|e| {
                                FileError {
                                    err: format!(
                                        "Fail to decrypt wallet, error: {}",
                                        e
                                    ),
                                }
                                .build()
                            })?,
                    )
                }
                (false, true) => {
                    let new_password =
                        prompt_password_stdout("Enter new password: ")
                            .map_err(|e| {
                                ParseError {
                                    err: format!(
                                        "Fail to parse new password, error: {}",
                                        e
                                    ),
                                }
                                .build()
                            })?;
                    let retype_password = prompt_password_stdout(
                        "Retype password: ",
                    )
                    .map_err(|e| {
                        ParseError {
                            err: format!(
                                "Fail to parse re-type password, error: {}",
                                e
                            ),
                        }
                        .build()
                    })?;

                    if new_password != retype_password {
                        return ParseError {
                            err: format!("Passwords don't match"),
                        }
                        .fail();
                    }

                    let (dir, need_symlink) =
                        if path.is_dir() {
                            (path, false)
                        } else {
                            (path.parent().ok_or(snafu::NoneError).context(
                            FileError {
                                err: "Fail to obtain directory to save wallet",
                            },
                        )?, true)
                        };

                    let new_wallet = LocalWallet::new_keystore(
                        &dir,
                        &mut thread_rng(),
                        &new_password,
                    )
                    .map_err(|e| {
                        FileError {
                            err: format!("Fail to create wallet, error: {}", e),
                        }
                        .build()
                    })?;

                    if need_symlink {
                        // create symlink for the new wallet file
                        std::os::unix::fs::symlink(dir.join(new_wallet.1), path)
                            .map_err(|e| {
                                FileError {
                                    err: format!(
                                        "Fail to create wallet symlink, error: {}",
                                        e
                                    ),
                                }
                                .build()
                            })?
                    }

                    Some(new_wallet.0)
                }
                _ => {
                    return FileError {
                        err: format!("Wallet file doesn't exist, do you want to create one? Set --wallet_create to true"),
                    }
                    .fail();
                }
            }
        } else if let Some(phrase) = mnemonic {
            Some(
                MnemonicBuilder::<English>::default()
                    .phrase(phrase.as_str())
                    .build()
                    .map_err(|e| {
                        ParseError {
                            err: format!(
                                "Fail to parse mnemonic, error: {}",
                                e
                            ),
                        }
                        .build()
                    })?,
            )
        } else {
            None
        };

        Ok(Config {
            contracts,
            url,
            ws_url,
            wallet,
        })
    }
}
