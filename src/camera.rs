//! ASI camera enumeration and device handles.
//!
//! [`Sdk::cameras`] lists every connected camera's [`CameraInfo`] without
//! opening it. [`Sdk::open_camera`] opens and initialises a camera, returning a
//! [`Camera`] RAII handle that closes the device on drop. With the `simulation`
//! feature a single fabricated `ASI2600MM-Pro-Simulated` camera is presented and
//! the SDK is never called.

#[cfg(not(feature = "simulation"))]
use crate::{asi_check, sys};
use crate::{AsiError, Error, Result, Sdk};

/// Bayer colour-filter pattern (`ASI_BAYER_PATTERN`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BayerPattern {
    /// `ASI_BAYER_RG`.
    Rg,
    /// `ASI_BAYER_BG`.
    Bg,
    /// `ASI_BAYER_GR`.
    Gr,
    /// `ASI_BAYER_GB`.
    Gb,
}

impl BayerPattern {
    #[cfg(not(feature = "simulation"))]
    #[must_use]
    fn from_raw(v: u32) -> Self {
        match v {
            0 => Self::Rg,
            1 => Self::Bg,
            2 => Self::Gr,
            _ => Self::Gb,
        }
    }
}

/// Safe view of `ASI_CAMERA_INFO`. Readable without opening the camera
/// (via [`Sdk::cameras`]); also cached on an open [`Camera`].
#[derive(Debug, Clone, PartialEq)]
pub struct CameraInfo {
    /// `CameraID` — the handle used by all per-camera SDK calls.
    pub id: i32,
    /// Model name, e.g. `"ZWO ASI2600MM Pro"`.
    pub name: String,
    /// Full sensor width in pixels.
    pub max_width: u32,
    /// Full sensor height in pixels.
    pub max_height: u32,
    /// Colour (Bayer) sensor when `true`, monochrome when `false`.
    pub is_color: bool,
    /// Bayer pattern (meaningful only when [`is_color`](Self::is_color)).
    pub bayer_pattern: BayerPattern,
    /// Supported symmetric binning factors (e.g. `[1, 2, 3, 4]`).
    pub supported_bins: Vec<u32>,
    /// Pixel size in micrometres.
    pub pixel_size_um: f64,
    /// Whether the camera has a mechanical shutter.
    pub has_mechanical_shutter: bool,
    /// Whether the camera exposes an ST4 guide port.
    pub has_st4_port: bool,
    /// Whether the camera is a cooled model.
    pub is_cooler_cam: bool,
    /// Whether the camera is a USB 3.0 device.
    pub is_usb3: bool,
    /// Native electrons-per-ADU (`ElecPerADU`).
    pub e_per_adu: f32,
    /// ADC bit depth (e.g. `16`, `14`, `12`).
    pub bit_depth: u32,
    /// Whether the camera supports the trigger (industrial) modes.
    pub is_trigger_cam: bool,
}

/// ASI control type (`ASI_CONTROL_TYPE`) — the imaging-relevant subset.
///
/// Unrecognised control types are preserved as [`ControlType::Other`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlType {
    /// `ASI_GAIN`.
    Gain,
    /// `ASI_EXPOSURE` (microseconds).
    Exposure,
    /// `ASI_OFFSET` (a.k.a. brightness).
    Offset,
    /// `ASI_TEMPERATURE` (sensor temperature, in 0.1 °C units).
    Temperature,
    /// `ASI_HIGH_SPEED_MODE`.
    HighSpeedMode,
    /// `ASI_COOLER_POWER_PERC`.
    CoolerPowerPerc,
    /// `ASI_TARGET_TEMP` (cooler set-point, whole °C).
    TargetTemp,
    /// `ASI_COOLER_ON`.
    CoolerOn,
    /// A control type outside the subset mapped here; carries the raw value.
    Other(i32),
}

impl ControlType {
    #[cfg(not(feature = "simulation"))]
    #[must_use]
    fn from_raw(v: i32) -> Self {
        match v {
            0 => Self::Gain,
            1 => Self::Exposure,
            5 => Self::Offset,
            8 => Self::Temperature,
            14 => Self::HighSpeedMode,
            15 => Self::CoolerPowerPerc,
            16 => Self::TargetTemp,
            17 => Self::CoolerOn,
            other => Self::Other(other),
        }
    }
}

/// Safe view of `ASI_CONTROL_CAPS` — one tunable control's range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlCaps {
    /// Control name, e.g. `"Gain"`.
    pub name: String,
    /// Which control this describes.
    pub control_type: ControlType,
    /// Minimum value.
    pub min: i64,
    /// Maximum value.
    pub max: i64,
    /// Default value.
    pub default: i64,
    /// Whether the control can be written (some, e.g. temperature, are read-only).
    pub is_writable: bool,
    /// Whether the control supports the SDK's auto mode.
    pub is_auto_supported: bool,
}

/// An open ASI camera. Closes the device on drop.
#[derive(Debug)]
pub struct Camera {
    info: CameraInfo,
}

impl Sdk {
    /// Enumerate every connected camera's [`CameraInfo`] without opening it.
    ///
    /// # Errors
    /// Returns [`Error::Asi`] if the SDK fails to read a camera's properties.
    pub fn cameras(&self) -> Result<Vec<CameraInfo>> {
        #[cfg(feature = "simulation")]
        let infos = (0..crate::SIM_CAMERA_COUNT)
            .map(|_| sim_camera_info())
            .collect();
        #[cfg(not(feature = "simulation"))]
        let infos = {
            let n = self.camera_count()?;
            (0..n)
                .map(|index| {
                    let idx =
                        i32::try_from(index).map_err(|_| Error::Asi(AsiError::InvalidIndex))?;
                    read_camera_property(idx)
                })
                .collect::<Result<Vec<_>>>()?
        };
        Ok(infos)
    }

    /// Open and initialise the camera at enumeration `index`.
    ///
    /// On the real path this calls `ASIOpenCamera` + `ASIInitCamera`; the
    /// returned [`Camera`] closes the device on drop.
    ///
    /// # Errors
    /// Returns [`Error::Asi`] if the index is out of range or the SDK fails to
    /// open/initialise the camera.
    pub fn open_camera(&self, index: usize) -> Result<Camera> {
        #[cfg(feature = "simulation")]
        let camera = {
            if index >= crate::SIM_CAMERA_COUNT {
                return Err(Error::Asi(AsiError::InvalidIndex));
            }
            Camera {
                info: sim_camera_info(),
            }
        };
        #[cfg(not(feature = "simulation"))]
        let camera = {
            let idx = i32::try_from(index).map_err(|_| Error::Asi(AsiError::InvalidIndex))?;
            let info = read_camera_property(idx)?;
            // SAFETY: `info.id` is a valid CameraID from enumeration; open it.
            asi_check(unsafe { sys::ASIOpenCamera(info.id) } as i32)?;
            // SAFETY: the camera was just opened; initialise it. On failure,
            // close it again so the open handle is not leaked.
            if let Err(e) = asi_check(unsafe { sys::ASIInitCamera(info.id) } as i32) {
                unsafe {
                    let _ = sys::ASICloseCamera(info.id);
                }
                return Err(e);
            }
            Camera { info }
        };
        Ok(camera)
    }
}

impl Camera {
    /// The camera's cached [`CameraInfo`].
    #[must_use]
    pub fn info(&self) -> &CameraInfo {
        &self.info
    }

    /// The camera's `CameraID`.
    #[must_use]
    pub fn id(&self) -> i32 {
        self.info.id
    }

    /// The camera's stable serial number as a 16-character hex string.
    ///
    /// Reads `ASIGetSerialNumber` (the 8-byte hardware serial); if the model
    /// reports none, falls back to the writable flash id (`ASIGetID`, USB3
    /// only). Requires the camera to be open, which it always is here.
    ///
    /// # Errors
    /// Returns [`Error::Asi`] if neither a serial nor a flash id is available.
    pub fn serial(&self) -> Result<String> {
        #[cfg(feature = "simulation")]
        let serial = SIM_SERIAL.to_owned();
        #[cfg(not(feature = "simulation"))]
        let serial = {
            // SAFETY: `ASI_SN` is a POD `[u8; 8]`; the SDK fills it on success.
            let mut sn: sys::ASI_SN = unsafe { std::mem::zeroed() };
            if asi_check(unsafe { sys::ASIGetSerialNumber(self.info.id, &mut sn) } as i32).is_ok() {
                hex8(&sn.id)
            } else {
                // SAFETY: as above; `ASIGetID` fills the 8-byte flash id.
                let mut fid: sys::ASI_ID = unsafe { std::mem::zeroed() };
                asi_check(unsafe { sys::ASIGetID(self.info.id, &mut fid) } as i32)?;
                hex8(&fid.id)
            }
        };
        Ok(serial)
    }

    /// Enumerate this camera's tunable controls and their ranges.
    ///
    /// # Errors
    /// Returns [`Error::Asi`] if the SDK fails to read the control list.
    pub fn control_caps(&self) -> Result<Vec<ControlCaps>> {
        #[cfg(feature = "simulation")]
        let caps = sim_control_caps();
        #[cfg(not(feature = "simulation"))]
        let caps = {
            let mut n: std::os::raw::c_int = 0;
            // SAFETY: `self.info.id` is an open camera; the SDK writes the count.
            asi_check(unsafe { sys::ASIGetNumOfControls(self.info.id, &mut n) } as i32)?;
            let count = usize::try_from(n).unwrap_or(0);
            (0..count)
                .map(|i| {
                    let idx = i32::try_from(i).map_err(|_| Error::Asi(AsiError::InvalidIndex))?;
                    // SAFETY: POD struct filled by the SDK for a valid index.
                    let mut raw: sys::ASI_CONTROL_CAPS = unsafe { std::mem::zeroed() };
                    asi_check(
                        unsafe { sys::ASIGetControlCaps(self.info.id, idx, &mut raw) } as i32,
                    )?;
                    Ok(control_caps_from_raw(&raw))
                })
                .collect::<Result<Vec<_>>>()?
        };
        Ok(caps)
    }
}

#[cfg(not(feature = "simulation"))]
impl Drop for Camera {
    fn drop(&mut self) {
        // SAFETY: closing an open camera by id; `ASICloseCamera` is idempotent
        // and returns success even if the camera is already closed.
        unsafe {
            let _ = sys::ASICloseCamera(self.info.id);
        }
    }
}

// ---- real FFI helpers --------------------------------------------------------

#[cfg(not(feature = "simulation"))]
fn read_camera_property(index: i32) -> Result<CameraInfo> {
    // SAFETY: `ASI_CAMERA_INFO` is POD; the SDK fills it for a valid index.
    let mut raw: sys::ASI_CAMERA_INFO = unsafe { std::mem::zeroed() };
    asi_check(unsafe { sys::ASIGetCameraProperty(&mut raw, index) } as i32)?;
    Ok(camera_info_from_raw(&raw))
}

#[cfg(not(feature = "simulation"))]
fn camera_info_from_raw(raw: &sys::ASI_CAMERA_INFO) -> CameraInfo {
    CameraInfo {
        id: raw.CameraID,
        name: c_string_field(&raw.Name),
        max_width: u32::try_from(raw.MaxWidth).unwrap_or(0),
        max_height: u32::try_from(raw.MaxHeight).unwrap_or(0),
        is_color: raw.IsColorCam != 0,
        bayer_pattern: BayerPattern::from_raw(raw.BayerPattern),
        supported_bins: raw
            .SupportedBins
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| u32::try_from(b).unwrap_or(0))
            .collect(),
        pixel_size_um: raw.PixelSize,
        has_mechanical_shutter: raw.MechanicalShutter != 0,
        has_st4_port: raw.ST4Port != 0,
        is_cooler_cam: raw.IsCoolerCam != 0,
        is_usb3: raw.IsUSB3Camera != 0,
        e_per_adu: raw.ElecPerADU,
        bit_depth: u32::try_from(raw.BitDepth).unwrap_or(0),
        is_trigger_cam: raw.IsTriggerCam != 0,
    }
}

#[cfg(not(feature = "simulation"))]
fn control_caps_from_raw(raw: &sys::ASI_CONTROL_CAPS) -> ControlCaps {
    ControlCaps {
        name: c_string_field(&raw.Name),
        control_type: ControlType::from_raw(raw.ControlType as i32),
        min: raw.MinValue,
        max: raw.MaxValue,
        default: raw.DefaultValue,
        is_writable: raw.IsWritable != 0,
        is_auto_supported: raw.IsAutoSupported != 0,
    }
}

/// Read a fixed-size, NUL-terminated C `char` buffer into an owned [`String`]
/// (lossy on invalid UTF-8). Portable across `c_char` signedness.
#[cfg(not(feature = "simulation"))]
fn c_string_field(buf: &[std::os::raw::c_char]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| (c as i32 & 0xff) as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Format an 8-byte hardware id as a 16-character lowercase hex string.
#[cfg(not(feature = "simulation"))]
fn hex8(bytes: &[u8; 8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---- simulation backend ------------------------------------------------------

#[cfg(feature = "simulation")]
const SIM_SERIAL: &str = "0a1b2c3d4e5f6071";

/// The fabricated simulated camera. Mirrors the rusty-photon `zwo-camera`
/// design-doc device: `ASI2600MM-Pro-Simulated` — 6248×4176 monochrome, 16-bit,
/// cooled, ST4 present.
#[cfg(feature = "simulation")]
fn sim_camera_info() -> CameraInfo {
    CameraInfo {
        id: 0,
        name: "ASI2600MM-Pro-Simulated".to_owned(),
        max_width: 6248,
        max_height: 4176,
        is_color: false,
        bayer_pattern: BayerPattern::Rg,
        supported_bins: vec![1, 2, 3, 4],
        pixel_size_um: 3.76,
        has_mechanical_shutter: false,
        has_st4_port: true,
        is_cooler_cam: true,
        is_usb3: true,
        e_per_adu: 0.25,
        bit_depth: 16,
        is_trigger_cam: false,
    }
}

#[cfg(feature = "simulation")]
fn sim_control_caps() -> Vec<ControlCaps> {
    let cap =
        |name: &str, control_type, min, max, default, is_writable, is_auto_supported| ControlCaps {
            name: name.to_owned(),
            control_type,
            min,
            max,
            default,
            is_writable,
            is_auto_supported,
        };
    vec![
        cap("Gain", ControlType::Gain, 0, 500, 100, true, true),
        cap(
            "Exposure",
            ControlType::Exposure,
            32,
            2_000_000_000,
            10_000,
            true,
            true,
        ),
        cap("Offset", ControlType::Offset, 0, 1000, 50, true, false),
        cap(
            "Temperature",
            ControlType::Temperature,
            -500,
            1000,
            0,
            false,
            false,
        ),
        cap("CoolerOn", ControlType::CoolerOn, 0, 1, 0, true, false),
        cap(
            "TargetTemp",
            ControlType::TargetTemp,
            -40,
            30,
            0,
            true,
            false,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cameras_enumerates() {
        let sdk = Sdk::new().unwrap();
        let cams = sdk.cameras().unwrap();
        #[cfg(feature = "simulation")]
        {
            assert_eq!(cams.len(), crate::SIM_CAMERA_COUNT);
            let info = &cams[0];
            assert_eq!(info.name, "ASI2600MM-Pro-Simulated");
            assert_eq!(info.max_width, 6248);
            assert_eq!(info.max_height, 4176);
            assert!(!info.is_color);
            assert_eq!(info.bit_depth, 16);
            assert!(info.is_cooler_cam);
            assert!(info.has_st4_port);
            assert_eq!(info.supported_bins, vec![1, 2, 3, 4]);
        }
        // Without the feature this calls the real SDK; with no hardware the list
        // is empty, but the call must still succeed.
        #[cfg(not(feature = "simulation"))]
        {
            let _ = cams;
        }
    }

    #[cfg(feature = "simulation")]
    #[test]
    fn open_camera_exposes_info_serial_and_controls() {
        let sdk = Sdk::new().unwrap();
        let cam = sdk.open_camera(0).unwrap();
        assert_eq!(cam.id(), 0);
        assert_eq!(cam.info().name, "ASI2600MM-Pro-Simulated");

        let serial = cam.serial().unwrap();
        assert_eq!(serial.len(), 16);
        assert!(serial.chars().all(|c| c.is_ascii_hexdigit()));

        let caps = cam.control_caps().unwrap();
        let gain = caps
            .iter()
            .find(|c| c.control_type == ControlType::Gain)
            .unwrap();
        assert_eq!(gain.max, 500);
        let exposure = caps
            .iter()
            .find(|c| c.control_type == ControlType::Exposure)
            .unwrap();
        assert_eq!(exposure.min, 32);
    }

    #[cfg(feature = "simulation")]
    #[test]
    fn open_camera_out_of_range_is_rejected() {
        let sdk = Sdk::new().unwrap();
        assert_eq!(
            sdk.open_camera(99).unwrap_err(),
            Error::Asi(AsiError::InvalidIndex)
        );
    }
}
