//! One module per screen, same order as the sidebar. Views read `Shared` and
//! queue `Act`s; per-view state that outlives a frame lives in `ViewState`.

pub mod board;
pub mod chain;
pub mod freq;
pub mod hardware;
pub mod hop;
pub mod learn;
pub mod modulation;
pub mod overview;
pub mod registers;
pub mod spectrum;
pub mod sweep;
