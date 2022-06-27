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
    use crate::asilib::structs::AsiID;

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIStartExposure(camera_id: i32) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIGetNumOfConnectedCameras() -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
	fn ASIGetID(camera_id: i32, asi_id: &mut AsiID) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
	fn ASISetID(camera_id: i32, asi_id: AsiID) -> i32;
    }

    pub fn start_exposure(camera_id: i32) {
        utils::check_error_code(
	    unsafe {
		ASIStartExposure(camera_id)
	    }
	);
    }

    pub fn get_num_of_connected_cameras() -> i32 {
        unsafe {
	    ASIGetNumOfConnectedCameras()
	}
    }

    pub fn get_cam_id(camera_id: i32, asi_id: &mut AsiID) {
	utils::check_error_code(
            unsafe {
		ASIGetID(camera_id, asi_id)
	    }
	);
    }

    pub fn set_cam_id(camera_id: i32, asi_id: AsiID) {
	utils::check_error_code(
            unsafe {
		ASISetID(camera_id, asi_id)
	    }
	);
    }

    pub mod structs {
        // Struct to manipulate the ASI ID
        #[derive(Debug)]
        #[repr(C)]
        pub struct AsiID {
            pub id: [u8; 8],
        }

	impl AsiID {
	    pub fn new() -> Self {
		Self {
		    id: [0; 8],
		}
	    }
	}

        // The main structure of the ZWO library, this struct is passed to the C function
        // and will contain READ-ONLY phisycal properties of the camera.
        #[derive(Debug)]
        #[repr(C)]
        pub struct AsiCameraInfo {
            // The name of the camera, you can display this to the UI
            pub name: [u8; 64],
            // This is used to control everything of the camera in other functions.Start from 0.
            pub camera_id: i32,
            // The max height of the camera
            pub max_height: i64,
            // The max width of the camera
            pub max_width: i64,
            // Is this a color camera?
            pub is_color_cam: i32,
            // The bayer pattern of the sensor
            pub bayer_pattern: i32,
            // Which types of binnings are supported, 1 means bin1 which is supported by every camera, 2 means bin 2 etc.. 0 is the end of supported binning method
            pub supported_bins: [i32; 16],
            // This array will content with the support output format type.IMG_END is the end of supported video format
            pub supported_video_format: [i32; 8],
            // The pixel size, be aware that is only one dimension, the pitch would be pixel_size * pixel_size
            pub pixel_size: f64,
            // Is there a mechanical shutter?
            pub mechanical_shutter: i32,
            // Is there any ST4 port on the camera?
            pub st4_port: i32,
            // Is there a cooling system?
            pub is_cooler_cam: i32,
            // Can this camera be used as USB3 hub?
            pub is_usb3_host: i32,
            // Does this camera support USB3?
            pub is_usb3_camera: i32,
            // Number of e-/ADU
            pub elec_per_adu: f32,
            // The bit depth of the sensor (Usually 12, 14 or 16)
            pub bit_depth: i32,
            pub is_trigger_cam: i32,
            // ZWO reserved
            pub unused: [u8; 16],
        }

        // struct the will be passed to the C function that stores the actual ROI set.
        #[derive(Copy, Clone)]
        pub struct ROIFormat {
            pub width: i32,
            pub height: i32,
            pub bin: i32,
            pub img_type: i32,
        }

        #[repr(C)]
        #[derive(Debug)]
        pub struct AsiControlCaps {
            // The name of the Control like Exposure, Gain etc..
            pub name: [u8; 64],
            // Description of this control
            pub description: [u8; 128],
            pub max_value: i64,
            pub min_value: i64,
            pub default_value: i64,
            // Support auto set 1, don't support 0
            pub is_auto_supported: i32,
            // Some control like temperature can only be read by some cameras
            pub is_writable: i32,
            // This is used to get value and set value of the control
            pub control_type: i32,
            pub unused: [u8; 32],
        }
    }
}
