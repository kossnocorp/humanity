//! Plant communities, renewable stocks and sustainable forage production.
//! Independent of fauna, human state, world topology and rendering.
pub mod growth;
pub mod rules;
pub mod stocks;
pub use growth::{Environment, advance};
pub use rules::{Community, FLORA};
pub use stocks::{Forage, Vegetation};

#[cfg(test)]
mod tests;
