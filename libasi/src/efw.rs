pub use libasi_sys::efw::*;
use log::error;

pub type EFWInfo = _EFW_INFO;
pub type EFWId = _EFW_ID;

fn check_error_code(code: i32) {
    match code {
	0 => (),
	1 => error!("EFW_ERROR_INVALID_INDEX"),
	2 => error!("EFW_ERROR_INVALID_ID"),
	3 => error!("EFW_ERROR_INVALID_VALUE"),
	// Failed to find the filter wheel, maybe the filter wheel has been removed
	4 => error!("EFW_ERROR_REMOVED"),
	// Filter wheel is moving
	5 => error!("EFW_ERROR_MOVING"),
	6 => error!("EFW_ERROR_ERROR_STATE"),
	7 => error!("EFW_ERROR_GENERAL_ERROR"),
	8 => error!("EFW_ERROR_NOT_SUPPORTED"),
	9 => error!("EFW_ERROR_CLOSED"),
	-1 => error!("EFW_ERROR_END"),
	_ => error!("UNKNOWN_ERROR"),
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

pub fn get_efw_id(index: i32, id: *mut i32) {
    check_error_code(
	unsafe { libasi_sys::efw::EFWGetID(index, id) }
    );
}

pub fn open_efw(id: i32) {
    check_error_code(
	unsafe { libasi_sys::efw::EFWOpen(id) }
    );
}

pub fn check_wheel_is_moving(id: i32) -> bool {
    let mut info = EFWInfo::new();
    let status = unsafe { libasi_sys::efw::EFWGetProperty(id, &mut info) };

    match status {
	5 => return true,
	_ => return false,
    };
}


pub fn get_efw_property(id: i32, info: *mut EFWInfo) {
    check_error_code(
	unsafe { libasi_sys::efw::EFWGetProperty(id, info) } 
    );
}

pub fn get_efw_position(id: i32) -> i32 {
    let mut position: i32 = 0;
    check_error_code(
	unsafe { libasi_sys::efw::EFWGetPosition(id, &mut position) }
    );
    // To have users dealing with non 0 indexed values, we simply add always 1 to
    // the 0 indexed position returned from the firmware
    position + 1
}

pub fn set_efw_position(id: i32, position: i32) {
    // To have users dealing with non 0 indexed values, we simply subtract always 1 to
    // the 0 indexed position wanted by the user
    let indexed_0_position = position -1;
    check_error_code(
	unsafe { libasi_sys::efw::EFWSetPosition(id, indexed_0_position) }
    );
}

pub fn set_unidirection(id: i32, flag: bool) {
    check_error_code(
	unsafe { EFWSetDirection(id, flag) }
    );
}

pub fn is_unidirectional(id: i32) -> bool {
    let mut unid: bool = false;
    check_error_code(
	unsafe { EFWGetDirection(id, &mut unid) }
    );
    unid
}

pub fn calibrate_wheel(id: i32) {
    check_error_code(
	unsafe { EFWCalibrate(id) }
    );
}

pub fn close_efw(id: i32) {
    check_error_code(unsafe { EFWClose(id) });
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
pub fn get_hw_error_code(id: i32) -> i32 {
    let mut err_code: i32 = 0;
    check_error_code(unsafe { libasi_sys::efw::EFWGetHWErrorCode(id, &mut err_code) });
    err_code
}

/// Retrieves the firmware version as `(major, minor, build)`.
pub fn get_firmware_version(id: i32) -> (u8, u8, u8) {
    let mut major: u8 = 0;
    let mut minor: u8 = 0;
    let mut build: u8 = 0;
    check_error_code(unsafe {
        libasi_sys::efw::EFWGetFirmwareVersion(id, &mut major, &mut minor, &mut build)
    });
    (major, minor, build)
}

/// Retrieves the serial number of the EFW device.
/// Returns `EFW_SN` which is an alias for `EFW_ID` (`[u8; 8]`).
/// Note: returns `EFW_ERROR_NOT_SUPPORTED` on older firmware.
pub fn get_serial_number(id: i32) -> EFWId {
    let mut sn = EFWId::new();
    check_error_code(unsafe { libasi_sys::efw::EFWGetSerialNumber(id, &mut sn) });
    sn
}

/// Writes an 8-byte alias ID to the EFW device flash.
pub fn set_id(id: i32, alias: EFWId) {
    check_error_code(unsafe { libasi_sys::efw::EFWSetID(id, alias) });
}
