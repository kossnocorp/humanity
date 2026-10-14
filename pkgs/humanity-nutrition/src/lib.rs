//! Food stocks and acquisition/consumption algorithms. No dependence on World or rendering.
pub mod consumption;
pub mod fishing;
pub mod foraging;
pub mod hunting;
pub mod rules;
pub mod stocks;
pub use rules::{FORAGING, encounter};
pub use stocks::*;
