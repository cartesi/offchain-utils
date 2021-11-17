use anyhow::{anyhow, Result};
use ethers::contract::Abigen;
use proc_macro2::{Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};
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
    let mut new_stream = TokenStream::new();

    for v in stream {
        match v {
            TokenTree::Group(group) => {
                let group_stream = replace_ethers_crates(group.stream());

                new_stream.extend([TokenTree::Group(Group::new(
                    group.delimiter(),
                    group_stream,
                ))]);
            }
            TokenTree::Ident(ident) => {
                if ident == "ethers" {
                    new_stream.extend([
                        TokenTree::Ident(Ident::new(
                            "offchain_core",
                            Span::call_site(),
                        )),
                        TokenTree::Punct(Punct::new(':', Spacing::Joint)),
                        TokenTree::Punct(Punct::new(':', Spacing::Alone)),
                    ]);
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
            use ethers::contract::something;

            #[hi::there]
            fn foo() {
                let whatever = "test";
            }
        };
        let expected_output = quote! {
            use offchain_core::ethers::contract::something;

            #[hi::there]
            fn foo() {
                let whatever = "test";
            }
        }
        .to_string();

        let actual_output = replace_ethers_crates(input).to_string();

        assert_eq!(expected_output, actual_output);
    }
}
