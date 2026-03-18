pub use libasi_sys::camera::*;

pub type AsiCameraInfo = _ASI_CAMERA_INFO;
pub type AsiControlCaps = _ASI_CONTROL_CAPS;
pub type AsiID = _ASI_ID;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ROIFormat {
    pub width: i32,
    pub height: i32,
    pub bin: i32,
    pub img_type: i32,
}

/// All error codes returned by the ASI camera SDK.
#[derive(Debug, PartialEq)]
pub enum AsiError {
    InvalidIndex,
    InvalidId,
    InvalidControlType,
    CameraClosed,
    CameraRemoved,
    InvalidPath,
    InvalidFileFormat,
    InvalidSize,
    InvalidImgType,
    OutOfBoundary,
    Timeout,
    InvalidSequence,
    BufferTooSmall,
    VideoModeActive,
    ExposureInProgress,
    GeneralError,
    InvalidMode,
    End,
    Unknown(i32),
}

impl std::fmt::Display for AsiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub(crate) fn map_error_code(code: i32) -> Result<(), AsiError> {
    match code {
        0 => Ok(()),
        1 => Err(AsiError::InvalidIndex),
        2 => Err(AsiError::InvalidId),
        3 => Err(AsiError::InvalidControlType),
        4 => Err(AsiError::CameraClosed),
        5 => Err(AsiError::CameraRemoved),
        6 => Err(AsiError::InvalidPath),
        7 => Err(AsiError::InvalidFileFormat),
        8 => Err(AsiError::InvalidSize),
        9 => Err(AsiError::InvalidImgType),
        10 => Err(AsiError::OutOfBoundary),
        11 => Err(AsiError::Timeout),
        12 => Err(AsiError::InvalidSequence),
        13 => Err(AsiError::BufferTooSmall),
        14 => Err(AsiError::VideoModeActive),
        15 => Err(AsiError::ExposureInProgress),
        16 => Err(AsiError::GeneralError),
        17 => Err(AsiError::InvalidMode),
        18 => Err(AsiError::End),
        e => Err(AsiError::Unknown(e)),
    }
}

/// Abstraction over the ASI camera hardware. Implement this trait to inject a
/// mock for unit testing without physical hardware.
pub trait CameraHardware: Send + Sync {
    fn get_num_of_connected_cameras(&self) -> i32;
    fn get_camera_info(&self, info: &mut AsiCameraInfo, index: i32) -> Result<(), AsiError>;
    fn open_camera(&self, index: i32) -> Result<(), AsiError>;
    fn init_camera(&self, index: i32) -> Result<(), AsiError>;
    fn close_camera(&self, index: i32) -> Result<(), AsiError>;
    fn get_num_of_controls(&self, index: i32) -> Result<i32, AsiError>;
    fn get_control_caps(
        &self,
        camera_id: i32,
        cap_index: i32,
        caps: &mut AsiControlCaps,
    ) -> Result<(), AsiError>;
    fn get_control_value(
        &self,
        camera_index: i32,
        control_type: i32,
    ) -> Result<i64, AsiError>;
    fn set_control_value(
        &self,
        camera_index: i32,
        control_type: i32,
        value: i64,
        is_auto_set: i32,
    ) -> Result<(), AsiError>;
    fn get_roi_format(&self, camera_id: i32) -> Result<ROIFormat, AsiError>;
    fn set_roi_format(&self, camera_id: i32, roi: ROIFormat) -> Result<(), AsiError>;
    fn get_cam_id(&self, camera_id: i32) -> Result<AsiID, AsiError>;
    fn set_cam_id(&self, camera_id: i32, asi_id: AsiID) -> Result<(), AsiError>;
    fn start_exposure(&self, camera_id: i32) -> Result<(), AsiError>;
    fn stop_exposure(&self, camera_id: i32) -> Result<(), AsiError>;
    fn exposure_status(&self, camera_id: i32) -> Result<u32, AsiError>;
    fn download_exposure(&self, camera_id: i32, buffer: &mut [u8]) -> Result<(), AsiError>;
    fn get_start_position(&self, cam_idx: i32) -> Result<(i32, i32), AsiError>;
    fn get_camera_mode(&self, cam_idx: i32) -> Result<i32, AsiError>;
}

/// Real hardware implementation that delegates to the ZWO ASI SDK via FFI.
pub struct RealCamera;

impl CameraHardware for RealCamera {
    fn get_num_of_connected_cameras(&self) -> i32 {
        unsafe { libasi_sys::camera::ASIGetNumOfConnectedCameras() }
    }

    fn get_camera_info(&self, info: &mut AsiCameraInfo, index: i32) -> Result<(), AsiError> {
        map_error_code(unsafe { libasi_sys::camera::ASIGetCameraProperty(info, index) })
    }

    fn open_camera(&self, index: i32) -> Result<(), AsiError> {
        map_error_code(unsafe { libasi_sys::camera::ASIOpenCamera(index) })
    }

    fn init_camera(&self, index: i32) -> Result<(), AsiError> {
        map_error_code(unsafe { libasi_sys::camera::ASIInitCamera(index) })
    }

    fn close_camera(&self, index: i32) -> Result<(), AsiError> {
        map_error_code(unsafe { libasi_sys::camera::ASICloseCamera(index) })
    }

    fn get_num_of_controls(&self, index: i32) -> Result<i32, AsiError> {
        let mut noc = 0i32;
        map_error_code(unsafe { libasi_sys::camera::ASIGetNumOfControls(index, &mut noc) })?;
        Ok(noc)
    }

    fn get_control_caps(
        &self,
        camera_id: i32,
        cap_index: i32,
        caps: &mut AsiControlCaps,
    ) -> Result<(), AsiError> {
        map_error_code(unsafe {
            libasi_sys::camera::ASIGetControlCaps(camera_id, cap_index, caps)
        })
    }

    fn get_control_value(
        &self,
        camera_index: i32,
        control_type: i32,
    ) -> Result<i64, AsiError> {
        let mut value: i64 = 0;
        let mut is_auto_set: i32 = 0;
        map_error_code(unsafe {
            libasi_sys::camera::ASIGetControlValue(
                camera_index,
                control_type,
                &mut value,
                &mut is_auto_set,
            )
        })?;
        Ok(value)
    }

    fn set_control_value(
        &self,
        camera_index: i32,
        control_type: i32,
        value: i64,
        is_auto_set: i32,
    ) -> Result<(), AsiError> {
        map_error_code(unsafe {
            libasi_sys::camera::ASISetControlValue(camera_index, control_type, value, is_auto_set)
        })
    }

    fn get_roi_format(&self, camera_id: i32) -> Result<ROIFormat, AsiError> {
        let mut width = 0i32;
        let mut height = 0i32;
        let mut bin = 0i32;
        let mut img_type = 0i32;
        map_error_code(unsafe {
            libasi_sys::camera::ASIGetROIFormat(
                camera_id,
                &mut width,
                &mut height,
                &mut bin,
                &mut img_type,
            )
        })?;
        Ok(ROIFormat {
            width,
            height,
            bin,
            img_type,
        })
    }

    fn set_roi_format(&self, camera_id: i32, roi: ROIFormat) -> Result<(), AsiError> {
        map_error_code(unsafe {
            libasi_sys::camera::ASISetROIFormat(
                camera_id, roi.width, roi.height, roi.bin, roi.img_type,
            )
        })
    }

    fn get_cam_id(&self, camera_id: i32) -> Result<AsiID, AsiError> {
        let mut id = AsiID::new();
        map_error_code(unsafe { libasi_sys::camera::ASIGetID(camera_id, &mut id) })?;
        Ok(id)
    }

    fn set_cam_id(&self, camera_id: i32, asi_id: AsiID) -> Result<(), AsiError> {
        map_error_code(unsafe { libasi_sys::camera::ASISetID(camera_id, asi_id) })
    }

    fn start_exposure(&self, camera_id: i32) -> Result<(), AsiError> {
        map_error_code(unsafe { libasi_sys::camera::ASIStartExposure(camera_id, 0) })
    }

    fn stop_exposure(&self, camera_id: i32) -> Result<(), AsiError> {
        map_error_code(unsafe { libasi_sys::camera::ASIStopExposure(camera_id) })
    }

    fn exposure_status(&self, camera_id: i32) -> Result<u32, AsiError> {
        let mut status: u32 = 0;
        map_error_code(unsafe {
            libasi_sys::camera::ASIGetExpStatus(camera_id, &mut status)
        })?;
        Ok(status)
    }

    fn download_exposure(&self, camera_id: i32, buffer: &mut [u8]) -> Result<(), AsiError> {
        map_error_code(unsafe {
            libasi_sys::camera::ASIGetDataAfterExp(
                camera_id,
                buffer.as_mut_ptr(),
                buffer.len() as i64,
            )
        })
    }

    fn get_start_position(&self, cam_idx: i32) -> Result<(i32, i32), AsiError> {
        let mut start_x = 0i32;
        let mut start_y = 0i32;
        map_error_code(unsafe {
            libasi_sys::camera::ASIGetStartPos(cam_idx, &mut start_x, &mut start_y)
        })?;
        Ok((start_x, start_y))
    }

    fn get_camera_mode(&self, cam_idx: i32) -> Result<i32, AsiError> {
        let mut mode = 0i32;
        map_error_code(unsafe { libasi_sys::camera::ASIGetCameraMode(cam_idx, &mut mode) })?;
        Ok(mode)
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
        assert_eq!(map_error_code(1), Err(AsiError::InvalidIndex));
        assert_eq!(map_error_code(2), Err(AsiError::InvalidId));
        assert_eq!(map_error_code(3), Err(AsiError::InvalidControlType));
        assert_eq!(map_error_code(4), Err(AsiError::CameraClosed));
        assert_eq!(map_error_code(5), Err(AsiError::CameraRemoved));
        assert_eq!(map_error_code(6), Err(AsiError::InvalidPath));
        assert_eq!(map_error_code(7), Err(AsiError::InvalidFileFormat));
        assert_eq!(map_error_code(8), Err(AsiError::InvalidSize));
        assert_eq!(map_error_code(9), Err(AsiError::InvalidImgType));
        assert_eq!(map_error_code(10), Err(AsiError::OutOfBoundary));
        assert_eq!(map_error_code(11), Err(AsiError::Timeout));
        assert_eq!(map_error_code(12), Err(AsiError::InvalidSequence));
        assert_eq!(map_error_code(13), Err(AsiError::BufferTooSmall));
        assert_eq!(map_error_code(14), Err(AsiError::VideoModeActive));
        assert_eq!(map_error_code(15), Err(AsiError::ExposureInProgress));
        assert_eq!(map_error_code(16), Err(AsiError::GeneralError));
        assert_eq!(map_error_code(17), Err(AsiError::InvalidMode));
        assert_eq!(map_error_code(18), Err(AsiError::End));
    }

    #[test]
    fn test_map_error_code_unknown_positive() {
        assert_eq!(map_error_code(99), Err(AsiError::Unknown(99)));
    }

    #[test]
    fn test_map_error_code_unknown_negative() {
        assert_eq!(map_error_code(-5), Err(AsiError::Unknown(-5)));
    }
}
