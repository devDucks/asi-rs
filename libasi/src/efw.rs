pub use libasi_sys::efw::*;
use log::{error};

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

/***************************************************************************
Descriptions:
get the product ID of each wheel, at first set pPIDs as 0 and get length and then malloc a buffer to load the PIDs

Paras:
int* pPIDs: pointer to array of PIDs

Return: length of the array.
***************************************************************************/
//EFW_API int EFWGetProductIDs(int* pPIDs);

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

/***************************************************************************
Descriptions:
close filter wheel

Paras:
int ID: the ID of filter wheel

Return: 
EFW_ERROR_INVALID_ID: invalid ID value
EFW_SUCCESS: operation succeeds
***************************************************************************/
pub fn close_efw(id: i32) {
    check_error_code(
	unsafe { EFWClose(id) }
    );
}

/***************************************************************************
Descriptions:
get version string, like "0, 4, 0824"
***************************************************************************/
//EFW_API char* EFWGetSDKVersion();


/***************************************************************************
Descriptions:
get hardware error code of filter wheel

Paras:
int ID: the ID of filter wheel

bool *pErrCode: pointer to error code .

Return: 
EFW_ERROR_INVALID_ID: invalid ID value
EFW_ERROR_CLOSED: not opened
EFW_SUCCESS: operation succeeds
***************************************************************************/
//EFW_API EFW_ERROR_CODE EFWGetHWErrorCode(int ID, int *pErrCode);

/***************************************************************************
Descriptions:
Get firmware version of filter wheel

Paras:
int ID: the ID of filter wheel

int *major, int *minor, int *build: pointer to value.

Return: 
EFW_ERROR_INVALID_ID: invalid ID value
EFW_ERROR_CLOSED: not opened
EFW_SUCCESS: operation succeeds
***************************************************************************/
//EFW_API	EFW_ERROR_CODE EFWGetFirmwareVersion(int ID, unsigned char *major, unsigned char *minor, unsigned char *build);

/***************************************************************************
Descriptions:
Get the serial number from a EFW

Paras:
int ID: the ID of focuser

EFW_SN* pSN: pointer to SN

Return: 
EFW_ERROR_INVALID_ID: invalid ID value
EFW_ERROR_CLOSED: not opened
EFW_ERROR_NOT_SUPPORTED: the firmware does not support serial number
EFW_SUCCESS: operation succeeds
***************************************************************************/
//EFW_API EFW_ERROR_CODE EFWGetSerialNumber(int ID, EFW_SN* pSN);

//EFW_API EFW_ERROR_CODE EFWSetID(int ID, EFW_ID alias);
