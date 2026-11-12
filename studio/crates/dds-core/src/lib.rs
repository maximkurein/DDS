//! DDS Studio core: everything about the АО НПЦ «ЭЛВИС» 1508ПЛ8Т that does not
//! depend on a UI. The desktop app (egui) and the web server both use
//! its `Engine`.

pub mod api;
pub mod bus;
pub mod model;
pub mod regs;
pub mod sim;
pub mod transport;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Link(String),
    #[error("{0}")]
    Invalid(String),
    #[error("неизвестная команда: {0}")]
    UnknownCommand(String),
    #[error("аргументы команды: {0}")]
    Args(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub use api::Engine;
