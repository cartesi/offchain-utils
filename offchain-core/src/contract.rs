use anyhow::{anyhow, Result};
use ethers::contract::Abigen;
use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use quote::quote;
use serde_json::Value;
use std::error;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

pub use {crate::contract_include as include, crate::contract_path as path};

/// Generates file path for the contract's bindings.
///
/// # Examples
/// The example below illustrates how to create a file during compile time (i.e.
/// in a build script).
///
/// ```no_run
/// # use std::fs::File;
/// # use offchain_core::contract;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut output = File::create(contract::path!("rollups_contract.rs"))?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! contract_path {
    ($path:expr) => {
        format!("{}/{}", std::env::var("OUT_DIR").unwrap(), $path)
    };
}

/// Includes contract's bindings.
///
/// # Examples
/// The example below includes a contract called `rollups_contract`. It has the
/// same effect as importing `rollups_contract::*`.
///
/// ```ignore
/// # use offchain_core::contract;
/// contract::include!("rollups_contract");
/// ```
#[macro_export]
macro_rules! contract_include {
    ($path:expr) => {
        include!(concat!(env!("OUT_DIR"), "/", $path, ".rs"));
    };
}

/// Generates type-safe contract bindings from a contract's ABI. Uses [`Abigen`]
/// under the hood.
///
/// # Examples
/// Usually you would put code similar to this in your `build.rs`.
///
/// ```no_run
/// # use std::fs::File;
/// # use offchain_core::contract;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let source = File::open("../../artifacts/contracts/RollupsImpl.sol/RollupsImpl.json")?;
/// let mut output = File::create(contract::path!("rollups_contract.rs"))?;
///
/// contract::write("RollupsImpl", source, output)?;
/// # Ok(())
/// # }
/// ```
///
/// [`Abigen`]: ethers::contract::Abigen
pub fn write<R, W>(
    contract_name: &str,
    source: R,
    mut output: W,
) -> Result<(), Box<dyn error::Error>>
where
    R: Read,
    W: Write,
{
    let source: Value = serde_json::from_reader(source)?;
    let abi_source = serde_json::to_string(&source["abi"])?;

    let bindings = Abigen::new(&contract_name, abi_source)?.generate()?;
    let tokens = bindings.into_tokens();

    let tokens = self::replace_ethers_crates(tokens);
    let raw = tokens.to_string();
    let formatted = self::format(&raw).unwrap_or(raw);

    output.write_all(formatted.as_bytes())?;

    Ok(())
}

/// Formats the raw input source string and return formatted output using
/// locally installed `rustfmt`.
fn format<S>(source: S) -> Result<String>
where
    S: AsRef<str>,
{
    let mut rustfmt = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    {
        let stdin = rustfmt.stdin.as_mut().ok_or_else(|| {
            anyhow!("stdin was not created for `rustfmt` child process")
        })?;
        stdin.write_all(source.as_ref().as_bytes())?;
    }

    let output = rustfmt.wait_with_output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "`rustfmt` exited with code {}:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout)
}

/// Changes ethers crates to the offchain_core reexport.
///
/// This way, updating the ethers version is trivial, and we make sure every
/// contract bindings use it.
fn replace_ethers_crates(stream: TokenStream) -> TokenStream {
    replace_ethers_crates_inner(stream, false)
}

fn replace_ethers_crates_inner(
    stream: TokenStream,
    inside_module: bool,
) -> TokenStream {
    let mut module = false;
    let mut inside_module = inside_module;
    let mut new_stream = TokenStream::new();

    for v in stream {
        match v {
            TokenTree::Group(group) => {
                let inside_module =
                    module && matches!(group.delimiter(), Delimiter::Brace);
                let group_stream =
                    replace_ethers_crates_inner(group.stream(), inside_module);
                if inside_module {
                    module = false;
                }
                new_stream.extend([TokenTree::Group(Group::new(
                    group.delimiter(),
                    group_stream,
                ))]);
            }
            TokenTree::Ident(ident) => {
                if ident == "use" && inside_module {
                    inside_module = false;
                    new_stream.extend([quote!(
                        mod ethers_core {
                            pub use offchain_core::ethers::core::*;
                        }
                        mod ethers_providers {
                            pub use offchain_core::ethers::providers::*;
                        }
                        mod ethers_contract {
                            pub use offchain_core::ethers::contract::*;
                        }
                    )]);
                }
                if ident == "mod" {
                    module = true;
                }

                new_stream.extend([TokenTree::Ident(ident)]);
            }
            tree => new_stream.extend([tree]),
        }
    }

    new_stream
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_replacing_ethers_crates_uses_correct_crates() {
        let input = quote! {
            mod prdel {
                use ethers::whatever;
            }
        };
        let expected_output = quote! {
            mod prdel {
                mod ethers_core {
                    pub use offchain_core::ethers::core::*;
                }
                mod ethers_providers {
                    pub use offchain_core::ethers::providers::*;
                }
                mod ethers_contract {
                    pub use offchain_core::ethers::contract::*;
                }
                use ethers::whatever;
            }
        }
        .to_string();

        let actual_output = replace_ethers_crates(input).to_string();

        assert_eq!(expected_output, actual_output);
    }
}
