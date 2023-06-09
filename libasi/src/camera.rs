pub use libasi_sys::camera::*;
use log::error;

pub type AsiCameraInfo = _ASI_CAMERA_INFO;
pub type AsiControlCaps = _ASI_CONTROL_CAPS;
pub type AsiID = _ASI_ID;

#[derive(Copy, Clone, Debug)]
pub struct ROIFormat {
    pub width: i32,
    pub height: i32,
    pub bin: i32,
    pub img_type: i32,
}

fn check_error_code(code: i32) {
    match code {
        // Success
        0 => (),
        // No camera connected or index value out of boundary
        1 => error!("ASI_ERROR_INVALID_INDEX"),
        2 => error!("ASI_ERROR_INVALID_ID"),
        3 => error!("ASI_ERROR_INVALID_CONTROL_TYPE"),
        // Camera didn't open
        4 => error!("ASI_ERROR_CAMERA_CLOSED"),
        // Failed to find the camera, maybe the camera has been removed
        5 => error!("ASI_ERROR_CAMERA_REMOVED"),
        // Cannot find the path of the file
        6 => error!("ASI_ERROR_INVALID_PATH"),
        7 => error!("ASI_ERROR_INVALID_FILEFORMAT"),
        // Wrong video format size
        8 => error!("ASI_ERROR_INVALID_SIZE"),
        9 => error!("ASI_ERROR_INVALID_IMGTYPE"), //unsupported image formate
        10 => error!("ASI_ERROR_OUTOF_BOUNDARY"), //the startpos is out of boundary
        // Communication timeout
        11 => error!("ASI_ERROR_TIMEOUT"),
        12 => error!("ASI_ERROR_INVALID_SEQUENCE"), //stop capture first!
        13 => error!("ASI_ERROR_BUFFER_TOO_SMALL"), //buffer size is not big enough
        14 => error!("ASI_ERROR_VIDEO_MODE_ACTIVE"),
        15 => error!("ASI_ERROR_EXPOSURE_IN_PROGRESS"),
        16 => error!("ASI_ERROR_GENERAL_ERROR"), //general error, eg: value is out of valid range
        17 => error!("ASI_ERROR_INVALID_MODE"),  //the current mode is wrong
        18 => error!("ASI_ERROR_END"),
        e => error!("unknown error {}", e),
    }
}

pub fn start_exposure(camera_id: i32) {
    check_error_code(unsafe { libasi_sys::camera::ASIStartExposure(camera_id, 0) });
}

pub fn stop_exposure(camera_id: i32) {
    check_error_code(unsafe { libasi_sys::camera::ASIStopExposure(camera_id) });
}

#[cfg(windows)]
pub fn exposure_status(camera_id: i32, status: *mut i32) {
    check_error_code(unsafe { libasi_sys::camera::ASIGetExpStatus(camera_id, status) });
}

#[cfg(unix)]
pub fn exposure_status(camera_id: i32, status: *mut u32) {
    check_error_code(unsafe { libasi_sys::camera::ASIGetExpStatus(camera_id, status) });
}

#[cfg(windows)]
pub fn download_exposure(camera_id: i32, buffer: *mut u8, buf_size: i32) {
    check_error_code(unsafe { libasi_sys::camera::ASIGetDataAfterExp(camera_id, buffer, buf_size) });
}

#[cfg(unix)]
pub fn download_exposure(camera_id: i32, buffer: *mut u8, buf_size: i64) {
    check_error_code(unsafe { libasi_sys::camera::ASIGetDataAfterExp(camera_id, buffer, buf_size) });
}

pub fn get_num_of_connected_cameras() -> i32 {
    unsafe { libasi_sys::camera::ASIGetNumOfConnectedCameras() }
}

pub fn get_cam_id(camera_id: i32, asi_id: *mut AsiID) {
    check_error_code(unsafe { libasi_sys::camera::ASIGetID(camera_id, asi_id) });
}

pub fn set_cam_id(camera_id: i32, asi_id: AsiID) {
    check_error_code(unsafe { libasi_sys::camera::ASISetID(camera_id, asi_id) });
}

pub fn open_camera(camera_index: i32) {
    check_error_code(unsafe { libasi_sys::camera::ASIOpenCamera(camera_index) });
}

pub fn init_camera(camera_index: i32) {
    check_error_code(unsafe { libasi_sys::camera::ASIInitCamera(camera_index) });
}

pub fn close_camera(camera_index: i32) {
    check_error_code(unsafe { libasi_sys::camera::ASICloseCamera(camera_index) });
}

pub fn get_control_caps(camera_id: i32, index: i32, noc: *mut AsiControlCaps) {
    check_error_code(unsafe { libasi_sys::camera::ASIGetControlCaps(camera_id, index, noc) });
}

pub fn get_num_of_controls(camera_index: i32, noc: *mut i32) {
    check_error_code(unsafe { libasi_sys::camera::ASIGetNumOfControls(camera_index, noc) });
}

pub fn get_camera_info(asi_info: *mut AsiCameraInfo, camera_index: i32) {
    check_error_code(unsafe { libasi_sys::camera::ASIGetCameraProperty(asi_info, camera_index) });
}

#[cfg(windows)]
pub fn get_control_value(
    camera_index: i32,
    control_type: i32,
    value: &mut i32,
    is_auto_set: &mut i32,
) {
    check_error_code(unsafe {
        libasi_sys::camera::ASIGetControlValue(camera_index, control_type, value, is_auto_set)
    });
}

#[cfg(unix)]
pub fn get_control_value(
    camera_index: i32,
    control_type: i32,
    value: &mut i64,
    is_auto_set: &mut i32,
) {
    check_error_code(unsafe {
        libasi_sys::camera::ASIGetControlValue(camera_index, control_type, value, is_auto_set)
    });
}

#[cfg(windows)]
pub fn set_control_value(camera_index: i32, control_type: i32, value: i32, is_auto_set: i32) {
    check_error_code(unsafe {
        libasi_sys::camera::ASISetControlValue(camera_index, control_type, value, is_auto_set)
    });
}

#[cfg(unix)]
pub fn set_control_value(camera_index: i32, control_type: i32, value: ::std::os::raw::c_long, is_auto_set: i32) {
    check_error_code(unsafe {
        libasi_sys::camera::ASISetControlValue(camera_index, control_type, value, is_auto_set)
    });
}

pub fn get_roi_format(
    camera_id: i32,
    width: &mut i32,
    height: &mut i32,
    bin: &mut i32,
    img_type: &mut i32,
) {
    check_error_code(unsafe {
        libasi_sys::camera::ASIGetROIFormat(camera_id, width, height, bin, img_type)
    });
}

pub fn set_roi_format(
    camera_id: i32,
    width: i32,
    height: i32,
    bin: i32,
    img_type: i32,
) {
    check_error_code(unsafe {
        libasi_sys::camera::ASISetROIFormat(camera_id, width, height, bin, img_type)
    });
}

pub fn get_start_position(cam_idx: i32, start_x: &mut i32, start_y: &mut i32) {
    check_error_code(unsafe {
        libasi_sys::camera::ASIGetStartPos(cam_idx, start_x, start_y)
    });
}

pub fn get_camera_mode(cam_idx: i32, camera_mode: &mut i32) {
    check_error_code(unsafe {
        libasi_sys::camera::ASIGetCameraMode(cam_idx, camera_mode)
    });
}
