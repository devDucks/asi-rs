pub mod utils {
    use log::error;

    pub fn check_error_code(code: i32) {
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
            12 => error!("ASI_ERROR_INVALID_SEQUENCE"), //stop capture first
            13 => error!("ASI_ERROR_BUFFER_TOO_SMALL"), //buffer size is not big enough
            14 => error!("ASI_ERROR_VIDEO_MODE_ACTIVE"),
            15 => error!("ASI_ERROR_EXPOSURE_IN_PROGRESS"),
            16 => error!("ASI_ERROR_GENERAL_ERROR"), //general error, eg: value is out of valid range
            17 => error!("ASI_ERROR_INVALID_MODE"),  //the current mode is wrong
            18 => error!("ASI_ERROR_END"),
            e => error!("unknown error {}", e),
        }
    }
}

pub mod asilib {
    use crate::utils;

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIStartExposure(camera_id: i32) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        pub fn ASIGetNumOfConnectedCameras() -> i32;
    }

    pub fn start_exposure(camera_id: i32) {
        utils::check_error_code(unsafe { ASIStartExposure(camera_id) });
    }

    pub fn get_num_of_connected_cameras() -> i32 {
        unsafe { ASIGetNumOfConnectedCameras() }
    }
}
