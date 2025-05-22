// lib.rs for executor crate
pub mod error;
pub mod handler;
pub mod formatter;
pub mod utils;

// Re-export key components to be easily accessible by users of this crate
pub use handler::*;
pub use error::*;

#[cfg(test)]
mod tests;
