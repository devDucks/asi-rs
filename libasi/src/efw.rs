pub use libasi_sys::efw::*;

pub type EFWInfo = _EFW_INFO;
pub type EFWId = _EFW_ID;

/// All error codes returned by the ASI EFW (filter wheel) SDK.
#[derive(Debug, PartialEq)]
pub enum EfwError {
    InvalidIndex,
    InvalidId,
    InvalidValue,
    Removed,
    Moving,
    ErrorState,
    GeneralError,
    NotSupported,
    Closed,
    End,
    Unknown(i32),
}

impl std::fmt::Display for EfwError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub(crate) fn map_error_code(code: i32) -> Result<(), EfwError> {
    match code {
        0 => Ok(()),
        1 => Err(EfwError::InvalidIndex),
        2 => Err(EfwError::InvalidId),
        3 => Err(EfwError::InvalidValue),
        4 => Err(EfwError::Removed),
        5 => Err(EfwError::Moving),
        6 => Err(EfwError::ErrorState),
        7 => Err(EfwError::GeneralError),
        8 => Err(EfwError::NotSupported),
        9 => Err(EfwError::Closed),
        -1 => Err(EfwError::End),
        e => Err(EfwError::Unknown(e)),
    }
}

/// Abstraction over the ASI EFW hardware. Implement this trait to inject a mock
/// for unit testing without physical hardware.
pub trait EfwHardware: Send + Sync {
    fn get_num_of_connected_devices(&self) -> i32;
    /// Returns the SDK-level device ID for the given enumeration index.
    fn get_id(&self, index: i32) -> Result<i32, EfwError>;
    fn open(&self, id: i32) -> Result<(), EfwError>;
    fn close(&self, id: i32) -> Result<(), EfwError>;
    fn get_property(&self, id: i32) -> Result<EFWInfo, EfwError>;
    /// Returns the current filter position (1-indexed, user-facing).
    fn get_position(&self, id: i32) -> Result<i32, EfwError>;
    /// Moves to the given filter position (1-indexed, user-facing).
    fn set_position(&self, id: i32, position: i32) -> Result<(), EfwError>;
    fn set_direction(&self, id: i32, unidirectional: bool) -> Result<(), EfwError>;
    fn get_direction(&self, id: i32) -> Result<bool, EfwError>;
    fn calibrate(&self, id: i32) -> Result<(), EfwError>;
    fn is_moving(&self, id: i32) -> bool;
}

/// Real hardware implementation that delegates to the ZWO EFW SDK via FFI.
pub struct RealEfw;

impl EfwHardware for RealEfw {
    fn get_num_of_connected_devices(&self) -> i32 {
        unsafe { libasi_sys::efw::EFWGetNum() }
    }

    fn get_id(&self, index: i32) -> Result<i32, EfwError> {
        let mut id: i32 = 0;
        map_error_code(unsafe { libasi_sys::efw::EFWGetID(index, &mut id) })?;
        Ok(id)
    }

    fn open(&self, id: i32) -> Result<(), EfwError> {
        map_error_code(unsafe { libasi_sys::efw::EFWOpen(id) })
    }

    fn close(&self, id: i32) -> Result<(), EfwError> {
        map_error_code(unsafe { libasi_sys::efw::EFWClose(id) })
    }

    fn get_property(&self, id: i32) -> Result<EFWInfo, EfwError> {
        let mut info = EFWInfo::new();
        map_error_code(unsafe { libasi_sys::efw::EFWGetProperty(id, &mut info) })?;
        Ok(info)
    }

    fn get_position(&self, id: i32) -> Result<i32, EfwError> {
        let mut position: i32 = 0;
        map_error_code(unsafe { libasi_sys::efw::EFWGetPosition(id, &mut position) })?;
        // Convert from 0-indexed SDK value to 1-indexed user-facing value.
        Ok(position + 1)
    }

    fn set_position(&self, id: i32, position: i32) -> Result<(), EfwError> {
        // Convert from 1-indexed user-facing value to 0-indexed SDK value.
        map_error_code(unsafe { libasi_sys::efw::EFWSetPosition(id, position - 1) })
    }

    fn set_direction(&self, id: i32, unidirectional: bool) -> Result<(), EfwError> {
        map_error_code(unsafe { EFWSetDirection(id, unidirectional) })
    }

    fn get_direction(&self, id: i32) -> Result<bool, EfwError> {
        let mut unid: bool = false;
        map_error_code(unsafe { EFWGetDirection(id, &mut unid) })?;
        Ok(unid)
    }

    fn calibrate(&self, id: i32) -> Result<(), EfwError> {
        map_error_code(unsafe { EFWCalibrate(id) })
    }

    fn is_moving(&self, id: i32) -> bool {
        let mut info = EFWInfo::new();
        // The SDK signals movement via return code 5 (EFW_ERROR_MOVING).
        let status = unsafe { libasi_sys::efw::EFWGetProperty(id, &mut info) };
        status == 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_error_code_success() {
        assert_eq!(map_error_code(0), Ok(()));
    }

    #[test]
    fn test_map_error_code_all_known_variants() {
        assert_eq!(map_error_code(1), Err(EfwError::InvalidIndex));
        assert_eq!(map_error_code(2), Err(EfwError::InvalidId));
        assert_eq!(map_error_code(3), Err(EfwError::InvalidValue));
        assert_eq!(map_error_code(4), Err(EfwError::Removed));
        assert_eq!(map_error_code(5), Err(EfwError::Moving));
        assert_eq!(map_error_code(6), Err(EfwError::ErrorState));
        assert_eq!(map_error_code(7), Err(EfwError::GeneralError));
        assert_eq!(map_error_code(8), Err(EfwError::NotSupported));
        assert_eq!(map_error_code(9), Err(EfwError::Closed));
        assert_eq!(map_error_code(-1), Err(EfwError::End));
    }

    #[test]
    fn test_map_error_code_unknown() {
        assert_eq!(map_error_code(99), Err(EfwError::Unknown(99)));
        assert_eq!(map_error_code(-99), Err(EfwError::Unknown(-99)));
    }
}
