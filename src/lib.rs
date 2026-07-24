//! Bluetooth and Bluetooth Low Energy receive and capture primitives.
//!
//! The default build deliberately has no third-party Rust dependencies. Native
//! SDR drivers will be isolated behind backend modules so the protocol and DSP
//! core remains testable without attached hardware.

pub mod advertising;
pub mod att;
pub mod backends;
pub mod ble;
pub mod capture;
pub mod complex;
mod crypto;
pub mod demod;
pub mod error;
pub mod iq;
pub mod l2cap;
pub mod link_layer;
pub mod ll_control;
pub mod native;
pub mod pcapng;
pub mod periodic;
pub mod sdr;
pub mod smp;

pub use error::{Error, Result};
