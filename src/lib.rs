//! # zwo-rs — safe Rust bindings for the ZWO ASI camera & EFW filter wheel SDK
//!
//! Sibling crate to [`qhyccd-rs`](https://crates.io/crates/qhyccd-rs). It wraps
//! the raw FFI in [`libzwo-sys`](https://crates.io/crates/libzwo-sys) — generated
//! by `bindgen` from the vendored MIT ZWO SDK headers — in a safe, ergonomic
//! API. It is consumed by rusty-photon's `zwo-camera` ASCOM Alpaca driver.
//!
//! ## Status
//!
//! **Skeleton.** The FFI is generated and links; the safe surface is being built
//! out per the rusty-photon `docs/plans/zwo-driver.md` plan. Scope order:
//! **Camera → EFW filter wheel → EAF focuser**.
//!
//! ## `simulation` feature
//!
//! Mirrors qhyccd-rs: enables a hardware-free, in-Rust simulated environment for
//! development and tests. Note (as with qhyccd-rs) the SDK is still *linked* when
//! this feature is enabled — it removes the camera, not the link.
//!
//! ## Build requirements
//!
//! - **libclang** — `libzwo-sys` runs `bindgen` at build time (needed for
//!   `check`/`clippy`/build; *not* the SDK).
//! - **The ZWO ASI SDK** (`libASICamera2`, `libEFWFilter`) + **libusb-1.0** on
//!   the link path — needed to *link* (i.e. `build`/`test`), even with the
//!   `simulation` feature. See the README.

/// Raw, unsafe FFI bindings (`bindgen`). Prefer the safe API in this crate.
pub use libzwo_sys as sys;

use thiserror::Error;

/// Errors returned by the safe API.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The requested operation is not yet implemented in this skeleton.
    #[error("operation not yet implemented")]
    NotImplemented,
    /// The underlying ASI/EFW SDK returned a non-success error code.
    #[error("ZWO SDK error (code {0})")]
    Sdk(i32),
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Entry point to the ZWO SDK (skeleton).
///
/// Enumerates connected ASI cameras and EFW filter wheels. With the `simulation`
/// feature a simulated environment is provided instead of real hardware.
#[derive(Debug, Default)]
pub struct Sdk {
    _private: (),
}

impl Sdk {
    /// Initialise the SDK.
    pub fn new() -> Result<Self> {
        tracing::debug!("initialising ZWO SDK (skeleton)");
        Ok(Self { _private: () })
    }

    /// Number of connected ASI cameras.
    ///
    /// TODO: wire to [`sys::ASIGetNumOfConnectedCameras`] (Camera phase).
    pub fn camera_count(&self) -> Result<usize> {
        Err(Error::NotImplemented)
    }

    /// Number of connected EFW filter wheels.
    ///
    /// TODO: wire to [`sys::EFWGetNum`] (EFW phase).
    pub fn filter_wheel_count(&self) -> Result<usize> {
        Err(Error::NotImplemented)
    }
}

#[cfg(feature = "simulation")]
pub mod simulation {
    //! Hardware-free, in-Rust simulation backend (no SDK calls).
    //!
    //! Skeleton: real simulated frames / EFW motion land with the Camera and
    //! filter-wheel phases.
    use rand::Rng;

    /// One 16-bit noise sample — a placeholder for simulated sensor frames.
    #[must_use]
    pub fn noise_sample() -> u16 {
        rand::rng().random()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_new_succeeds() {
        let sdk = Sdk::new().unwrap();
        // Skeleton: enumeration is not wired yet.
        assert!(matches!(sdk.camera_count(), Err(Error::NotImplemented)));
        assert!(matches!(
            sdk.filter_wheel_count(),
            Err(Error::NotImplemented)
        ));
    }

    #[cfg(feature = "simulation")]
    #[test]
    fn simulation_noise_sample_runs() {
        // Just exercise the simulation path; any u16 is valid.
        let _ = simulation::noise_sample();
    }
}
