pub use libasi_sys::efw::*;

pub type EFWInfo = _EFW_INFO;
pub type EFWId = _EFW_ID;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AsiEfwError {
    #[error("invalid index")]
    InvalidIndex,
    #[error("invalid id")]
    InvalidId,
    #[error("invalid value")]
    InvalidValue,
    #[error("filter wheel removed")]
    Removed,
    #[error("filter wheel is moving")]
    Moving,
    #[error("error state")]
    ErrorState,
    #[error("general error")]
    GeneralError,
    #[error("not supported")]
    NotSupported,
    #[error("device closed")]
    Closed,
    #[error("end sentinel")]
    End,
    #[error("unknown error code: {0}")]
    Unknown(i32),
}

fn check_error_code(code: i32) -> Result<(), AsiEfwError> {
    match code {
        0 => Ok(()),
        1 => Err(AsiEfwError::InvalidIndex),
        2 => Err(AsiEfwError::InvalidId),
        3 => Err(AsiEfwError::InvalidValue),
        4 => Err(AsiEfwError::Removed),
        5 => Err(AsiEfwError::Moving),
        6 => Err(AsiEfwError::ErrorState),
        7 => Err(AsiEfwError::GeneralError),
        8 => Err(AsiEfwError::NotSupported),
        9 => Err(AsiEfwError::Closed),
        -1 => Err(AsiEfwError::End),
        n => Err(AsiEfwError::Unknown(n)),
    }
}

pub fn get_num_of_connected_devices() -> i32 {
    unsafe { libasi_sys::efw::EFWGetNum() }
}

/// Returns the product IDs of all connected EFW devices.
/// Deprecated by the SDK — prefer `EFWCheck` when available.
pub fn get_product_ids() -> Vec<i32> {
    let len = unsafe { libasi_sys::efw::EFWGetProductIDs(std::ptr::null_mut()) };
    if len <= 0 {
        return Vec::new();
    }
    let mut pids = vec![0i32; len as usize];
    unsafe { libasi_sys::efw::EFWGetProductIDs(pids.as_mut_ptr()) };
    pids
}

pub fn get_efw_id(index: i32, id: *mut i32) -> Result<(), AsiEfwError> {
    check_error_code(unsafe { libasi_sys::efw::EFWGetID(index, id) })
}

pub fn open_efw(id: i32) -> Result<(), AsiEfwError> {
    check_error_code(unsafe { libasi_sys::efw::EFWOpen(id) })
}

pub fn check_wheel_is_moving(id: i32) -> bool {
    let mut info = EFWInfo::new();
    let status = unsafe { libasi_sys::efw::EFWGetProperty(id, &mut info) };
    matches!(check_error_code(status), Err(AsiEfwError::Moving))
}

pub fn get_efw_property(id: i32, info: *mut EFWInfo) -> Result<(), AsiEfwError> {
    check_error_code(unsafe { libasi_sys::efw::EFWGetProperty(id, info) })
}

pub fn get_efw_position(id: i32) -> Result<i32, AsiEfwError> {
    let mut position: i32 = 0;
    check_error_code(unsafe { libasi_sys::efw::EFWGetPosition(id, &mut position) })?;
    // SDK uses 0-based positions; callers work with 1-based. Return 0 while moving.
    Ok(position + 1)
}

pub fn set_efw_position(id: i32, position: i32) -> Result<(), AsiEfwError> {
    // Callers supply 1-based positions; SDK expects 0-based.
    let indexed_0_position = position - 1;
    check_error_code(unsafe { libasi_sys::efw::EFWSetPosition(id, indexed_0_position) })
}

pub fn set_unidirection(id: i32, flag: bool) -> Result<(), AsiEfwError> {
    check_error_code(unsafe { EFWSetDirection(id, flag) })
}

pub fn is_unidirectional(id: i32) -> Result<bool, AsiEfwError> {
    let mut unid: bool = false;
    check_error_code(unsafe { EFWGetDirection(id, &mut unid) })?;
    Ok(unid)
}

pub fn calibrate_wheel(id: i32) -> Result<(), AsiEfwError> {
    check_error_code(unsafe { EFWCalibrate(id) })
}

pub fn close_efw(id: i32) -> Result<(), AsiEfwError> {
    check_error_code(unsafe { EFWClose(id) })
}

/// Returns the SDK version string, e.g. `"1, 8, 4"`.
pub fn get_sdk_version() -> String {
    let ptr = unsafe { libasi_sys::efw::EFWGetSDKVersion() };
    if ptr.is_null() {
        return String::from("UNKNOWN");
    }
    unsafe { std::ffi::CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

/// Retrieves the hardware-level firmware error code for the given device.
pub fn get_hw_error_code(id: i32) -> Result<i32, AsiEfwError> {
    let mut err_code: i32 = 0;
    check_error_code(unsafe { libasi_sys::efw::EFWGetHWErrorCode(id, &mut err_code) })?;
    Ok(err_code)
}

/// Retrieves the firmware version as `(major, minor, build)`.
pub fn get_firmware_version(id: i32) -> Result<(u8, u8, u8), AsiEfwError> {
    let mut major: u8 = 0;
    let mut minor: u8 = 0;
    let mut build: u8 = 0;
    check_error_code(unsafe {
        libasi_sys::efw::EFWGetFirmwareVersion(id, &mut major, &mut minor, &mut build)
    })?;
    Ok((major, minor, build))
}

/// Retrieves the serial number of the EFW device.
/// Returns `EFW_SN` which is an alias for `EFW_ID` (`[u8; 8]`).
/// Note: returns `EFW_ERROR_NOT_SUPPORTED` on older firmware.
pub fn get_serial_number(id: i32) -> Result<EFWId, AsiEfwError> {
    let mut sn = EFWId::new();
    check_error_code(unsafe { libasi_sys::efw::EFWGetSerialNumber(id, &mut sn) })?;
    Ok(sn)
}

/// Writes an 8-byte alias ID to the EFW device flash.
pub fn set_id(id: i32, alias: EFWId) -> Result<(), AsiEfwError> {
    check_error_code(unsafe { libasi_sys::efw::EFWSetID(id, alias) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_code_is_ok() {
        assert!(check_error_code(0).is_ok());
    }

    #[test]
    fn all_known_efw_codes_map_correctly() {
        assert_eq!(check_error_code(1), Err(AsiEfwError::InvalidIndex));
        assert_eq!(check_error_code(2), Err(AsiEfwError::InvalidId));
        assert_eq!(check_error_code(3), Err(AsiEfwError::InvalidValue));
        assert_eq!(check_error_code(4), Err(AsiEfwError::Removed));
        assert_eq!(check_error_code(5), Err(AsiEfwError::Moving));
        assert_eq!(check_error_code(6), Err(AsiEfwError::ErrorState));
        assert_eq!(check_error_code(7), Err(AsiEfwError::GeneralError));
        assert_eq!(check_error_code(8), Err(AsiEfwError::NotSupported));
        assert_eq!(check_error_code(9), Err(AsiEfwError::Closed));
        assert_eq!(check_error_code(-1), Err(AsiEfwError::End));
    }

    #[test]
    fn moving_code_maps_to_moving_error() {
        assert_eq!(check_error_code(5), Err(AsiEfwError::Moving));
    }

    #[test]
    fn unknown_code_wraps_value() {
        assert_eq!(check_error_code(99), Err(AsiEfwError::Unknown(99)));
        assert_eq!(check_error_code(-99), Err(AsiEfwError::Unknown(-99)));
    }
}
