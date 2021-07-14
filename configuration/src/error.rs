use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    #[snafu(display("File error: {}", err))]
    FileError { err: String },
    #[snafu(display("Parse error: {}", err))]
    ParseError { err: String },
}

pub type Result<T> = std::result::Result<T, Error>;
