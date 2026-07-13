//! Bluetooth and Bluetooth Low Energy receive and capture primitives.
//!
//! The default build deliberately has no third-party Rust dependencies. Native
//! SDR drivers will be isolated behind backend modules so the protocol and DSP
//! core remains testable without attached hardware.

pub mod ble;
pub mod complex;
pub mod demod;
pub mod error;
pub mod iq;
pub mod sdr;

pub use error::{Error, Result};
