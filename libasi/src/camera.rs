pub use libasi_sys::camera::*;

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

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AsiCameraError {
    #[error("invalid camera index")]
    InvalidIndex,
    #[error("invalid camera id")]
    InvalidId,
    #[error("invalid control type")]
    InvalidControlType,
    #[error("camera not open")]
    CameraClosed,
    #[error("camera removed")]
    CameraRemoved,
    #[error("invalid path")]
    InvalidPath,
    #[error("invalid file format")]
    InvalidFileFormat,
    #[error("invalid size")]
    InvalidSize,
    #[error("invalid image type")]
    InvalidImgType,
    #[error("start position out of boundary")]
    OutOfBoundary,
    #[error("communication timeout")]
    Timeout,
    #[error("invalid sequence — stop capture first")]
    InvalidSequence,
    #[error("buffer too small")]
    BufferTooSmall,
    #[error("video mode active")]
    VideoModeActive,
    #[error("exposure in progress")]
    ExposureInProgress,
    #[error("general error")]
    GeneralError,
    #[error("invalid mode")]
    InvalidMode,
    #[error("end sentinel")]
    End,
    #[error("unknown error code: {0}")]
    Unknown(i32),
}

fn check_error_code(code: i32) -> Result<(), AsiCameraError> {
    match code {
        0 => Ok(()),
        1 => Err(AsiCameraError::InvalidIndex),
        2 => Err(AsiCameraError::InvalidId),
        3 => Err(AsiCameraError::InvalidControlType),
        4 => Err(AsiCameraError::CameraClosed),
        5 => Err(AsiCameraError::CameraRemoved),
        6 => Err(AsiCameraError::InvalidPath),
        7 => Err(AsiCameraError::InvalidFileFormat),
        8 => Err(AsiCameraError::InvalidSize),
        9 => Err(AsiCameraError::InvalidImgType),
        10 => Err(AsiCameraError::OutOfBoundary),
        11 => Err(AsiCameraError::Timeout),
        12 => Err(AsiCameraError::InvalidSequence),
        13 => Err(AsiCameraError::BufferTooSmall),
        14 => Err(AsiCameraError::VideoModeActive),
        15 => Err(AsiCameraError::ExposureInProgress),
        16 => Err(AsiCameraError::GeneralError),
        17 => Err(AsiCameraError::InvalidMode),
        18 => Err(AsiCameraError::End),
        n => Err(AsiCameraError::Unknown(n)),
    }
}

pub fn start_exposure(camera_id: i32) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIStartExposure(camera_id, 0) })
}

pub fn stop_exposure(camera_id: i32) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIStopExposure(camera_id) })
}

#[cfg(windows)]
pub fn exposure_status(camera_id: i32, status: *mut i32) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIGetExpStatus(camera_id, status) })
}

#[cfg(unix)]
pub fn exposure_status(camera_id: i32, status: *mut u32) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIGetExpStatus(camera_id, status) })
}

#[cfg(windows)]
pub fn download_exposure(
    camera_id: i32,
    buffer: *mut u8,
    buf_size: i32,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe {
        libasi_sys::camera::ASIGetDataAfterExp(camera_id, buffer, buf_size)
    })
}

#[cfg(unix)]
pub fn download_exposure(
    camera_id: i32,
    buffer: *mut u8,
    buf_size: i64,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe {
        libasi_sys::camera::ASIGetDataAfterExp(camera_id, buffer, buf_size)
    })
}

pub fn get_num_of_connected_cameras() -> i32 {
    unsafe { libasi_sys::camera::ASIGetNumOfConnectedCameras() }
}

pub fn get_cam_id(camera_id: i32, asi_id: *mut AsiID) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIGetID(camera_id, asi_id) })
}

pub fn set_cam_id(camera_id: i32, asi_id: AsiID) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASISetID(camera_id, asi_id) })
}

pub fn open_camera(camera_index: i32) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIOpenCamera(camera_index) })
}

pub fn init_camera(camera_index: i32) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIInitCamera(camera_index) })
}

pub fn close_camera(camera_index: i32) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASICloseCamera(camera_index) })
}

pub fn get_control_caps(
    camera_id: i32,
    index: i32,
    noc: *mut AsiControlCaps,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIGetControlCaps(camera_id, index, noc) })
}

pub fn get_num_of_controls(camera_index: i32, noc: *mut i32) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIGetNumOfControls(camera_index, noc) })
}

pub fn get_camera_info(
    asi_info: *mut AsiCameraInfo,
    camera_index: i32,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIGetCameraProperty(asi_info, camera_index) })
}

#[cfg(windows)]
pub fn get_control_value(
    camera_index: i32,
    control_type: i32,
    value: &mut i32,
    is_auto_set: &mut i32,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe {
        libasi_sys::camera::ASIGetControlValue(camera_index, control_type, value, is_auto_set)
    })
}

#[cfg(unix)]
pub fn get_control_value(
    camera_index: i32,
    control_type: i32,
    value: &mut i64,
    is_auto_set: &mut i32,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe {
        libasi_sys::camera::ASIGetControlValue(camera_index, control_type, value, is_auto_set)
    })
}

#[cfg(windows)]
pub fn set_control_value(
    camera_index: i32,
    control_type: i32,
    value: i32,
    is_auto_set: i32,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe {
        libasi_sys::camera::ASISetControlValue(camera_index, control_type, value, is_auto_set)
    })
}

#[cfg(unix)]
pub fn set_control_value(
    camera_index: i32,
    control_type: i32,
    value: ::std::os::raw::c_long,
    is_auto_set: i32,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe {
        libasi_sys::camera::ASISetControlValue(camera_index, control_type, value, is_auto_set)
    })
}

pub fn get_roi_format(
    camera_id: i32,
    width: &mut i32,
    height: &mut i32,
    bin: &mut i32,
    img_type: &mut i32,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe {
        libasi_sys::camera::ASIGetROIFormat(camera_id, width, height, bin, img_type)
    })
}

pub fn set_roi_format(
    camera_id: i32,
    width: i32,
    height: i32,
    bin: i32,
    img_type: i32,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe {
        libasi_sys::camera::ASISetROIFormat(camera_id, width, height, bin, img_type)
    })
}

pub fn get_start_position(
    cam_idx: i32,
    start_x: &mut i32,
    start_y: &mut i32,
) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIGetStartPos(cam_idx, start_x, start_y) })
}

pub fn get_camera_mode(cam_idx: i32, camera_mode: &mut i32) -> Result<(), AsiCameraError> {
    check_error_code(unsafe { libasi_sys::camera::ASIGetCameraMode(cam_idx, camera_mode) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_code_is_ok() {
        assert!(check_error_code(0).is_ok());
    }

    #[test]
    fn known_error_codes_map_correctly() {
        assert_eq!(check_error_code(1), Err(AsiCameraError::InvalidIndex));
        assert_eq!(check_error_code(2), Err(AsiCameraError::InvalidId));
        assert_eq!(check_error_code(3), Err(AsiCameraError::InvalidControlType));
        assert_eq!(check_error_code(4), Err(AsiCameraError::CameraClosed));
        assert_eq!(check_error_code(5), Err(AsiCameraError::CameraRemoved));
        assert_eq!(check_error_code(6), Err(AsiCameraError::InvalidPath));
        assert_eq!(check_error_code(7), Err(AsiCameraError::InvalidFileFormat));
        assert_eq!(check_error_code(8), Err(AsiCameraError::InvalidSize));
        assert_eq!(check_error_code(9), Err(AsiCameraError::InvalidImgType));
        assert_eq!(check_error_code(10), Err(AsiCameraError::OutOfBoundary));
        assert_eq!(check_error_code(11), Err(AsiCameraError::Timeout));
        assert_eq!(check_error_code(12), Err(AsiCameraError::InvalidSequence));
        assert_eq!(check_error_code(13), Err(AsiCameraError::BufferTooSmall));
        assert_eq!(check_error_code(14), Err(AsiCameraError::VideoModeActive));
        assert_eq!(check_error_code(15), Err(AsiCameraError::ExposureInProgress));
        assert_eq!(check_error_code(16), Err(AsiCameraError::GeneralError));
        assert_eq!(check_error_code(17), Err(AsiCameraError::InvalidMode));
        assert_eq!(check_error_code(18), Err(AsiCameraError::End));
    }

    #[test]
    fn unknown_code_wraps_value() {
        assert_eq!(check_error_code(99), Err(AsiCameraError::Unknown(99)));
        assert_eq!(check_error_code(-5), Err(AsiCameraError::Unknown(-5)));
    }
}
