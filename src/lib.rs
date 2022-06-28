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
    use crate::asilib::structs::{AsiCameraInfo, AsiControlCaps, AsiID};
    use crate::utils;

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

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIStopExposure(camera_id: i32) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIGetExpStatus(camera_id: i32, p_status: &mut i32) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIGetDataAfterExp(camera_id: i32, buffer: *mut libc::c_uchar, buf_size: i64) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIOpenCamera(camera_index: i32) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIInitCamera(camera_index: i32) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIGetControlCaps(camera_id: i32, index: i32, noc: *mut AsiControlCaps) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASICloseCamera(camera_index: i32) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIGetNumOfControls(camera_id: i32, noc: *mut i32) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIGetCameraProperty(asi_info: &mut AsiCameraInfo, camera_index: i32) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIGetROIFormat(
            camera_id: i32,
            width: &mut i32,
            height: &mut i32,
            bin: &mut i32,
            img_type: &mut i32,
        ) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASIGetControlValue(
            camera_id: i32,
            control_type: i32,
            value: &mut i64,
            is_auto_set: &mut i32,
        ) -> i32;
    }

    #[link(name = "ASICamera2")]
    extern "C" {
        fn ASISetControlValue(
            camera_index: i32,
            control_type: i32,
            value: i64,
            is_auto_set: i32,
        ) -> i32;
    }

    pub fn start_exposure(camera_id: i32) {
        utils::check_error_code(unsafe { ASIStartExposure(camera_id) });
    }

    pub fn stop_exposure(camera_id: i32) {
        utils::check_error_code(unsafe { ASIStopExposure(camera_id) });
    }

    pub fn exposure_status(camera_id: i32, status: &mut i32) {
        utils::check_error_code(unsafe { ASIGetExpStatus(camera_id, status) });
    }

    pub fn download_exposure(camera_id: i32, buffer: *mut u8, buf_size: i64) {
        utils::check_error_code(unsafe { ASIGetDataAfterExp(camera_id, buffer, buf_size) });
    }

    pub fn get_num_of_connected_cameras() -> i32 {
        unsafe { ASIGetNumOfConnectedCameras() }
    }

    pub fn get_cam_id(camera_id: i32, asi_id: &mut AsiID) {
        utils::check_error_code(unsafe { ASIGetID(camera_id, asi_id) });
    }

    pub fn set_cam_id(camera_id: i32, asi_id: AsiID) {
        utils::check_error_code(unsafe { ASISetID(camera_id, asi_id) });
    }

    pub fn open_camera(camera_index: i32) {
        utils::check_error_code(unsafe { ASIOpenCamera(camera_index) });
    }

    pub fn init_camera(camera_index: i32) {
        utils::check_error_code(unsafe { ASIInitCamera(camera_index) });
    }

    pub fn close_camera(camera_index: i32) {
        utils::check_error_code(unsafe { ASICloseCamera(camera_index) });
    }

    pub fn get_control_caps(camera_id: i32, index: i32, noc: *mut AsiControlCaps) {
        utils::check_error_code(unsafe { ASIGetControlCaps(camera_id, index, noc) });
    }

    pub fn get_num_of_controls(camera_index: i32, noc: *mut i32) {
        utils::check_error_code(unsafe { ASIGetNumOfControls(camera_index, noc) });
    }

    pub fn get_camera_info(asi_info: &mut AsiCameraInfo, camera_index: i32) {
        utils::check_error_code(unsafe { ASIGetCameraProperty(asi_info, camera_index) });
    }

    pub fn get_control_value(
        camera_index: i32,
        control_type: i32,
        value: &mut i64,
        is_auto_set: &mut i32,
    ) {
        utils::check_error_code(unsafe {
            ASIGetControlValue(camera_index, control_type, value, is_auto_set)
        });
    }

    pub fn set_control_value(camera_index: i32, control_type: i32, value: i64, is_auto_set: i32) {
        utils::check_error_code(unsafe {
            ASISetControlValue(camera_index, control_type, value, is_auto_set)
        });
    }

    pub fn get_roi_format(
        camera_id: i32,
        width: &mut i32,
        height: &mut i32,
        bin: &mut i32,
        img_type: &mut i32,
    ) {
        utils::check_error_code(unsafe {
            ASIGetROIFormat(camera_id, width, height, bin, img_type)
        });
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
                Self { id: [0; 8] }
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

        impl AsiCameraInfo {
            pub fn new() -> Self {
                Self {
                    name: [0; 64],
                    camera_id: 0,
                    max_height: 0,
                    max_width: 0,
                    is_color_cam: 1,
                    bayer_pattern: 1,
                    supported_bins: [0; 16],
                    supported_video_format: [0; 8],
                    pixel_size: 0.0,
                    mechanical_shutter: 0,
                    st4_port: 0,
                    is_cooler_cam: 0,
                    is_usb3_host: 0,
                    is_usb3_camera: 0,
                    elec_per_adu: 0.0,
                    bit_depth: 0,
                    is_trigger_cam: 0,
                    unused: [0; 16],
                }
            }
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

        impl AsiControlCaps {
            pub fn new() -> Self {
                Self {
                    name: [0; 64],
                    description: [0; 128],
                    max_value: 0,
                    min_value: 0,
                    default_value: 0,
                    is_auto_supported: 0,
                    is_writable: 0,
                    control_type: 0,
                    unused: [0; 32],
                }
            }
        }

        #[repr(C)]
        pub enum AsiControlType {
            AsiGain = 0,
            AsiExposure,
            AsiGamma,
            AsiWbR,
            AsiWbB,
            AsiOffset,
            AsiBandwidthoverload,
            AsiOverclock,
            // Returns 10*temperature
            AsiTemperature,
            AsiFlip,
            AsiAautoMaxGain,
            // In micro second
            AsiAutoMaxExp,
            // Target brightness
            AsiAutoTargetBrightness,
            AsiHardwareBin,
            AsiHighSpeedMode,
            AsiCoolerPowerPerc,
            // Do not need *10
            AsiTargetTemp,
            AsiCoolerOn,
            // Leads to less grid at software bin mode for color camera
            AsiMonoBin,
            AsiFanOn,
            AsiPatternAdjust,
            AsiAntiDewHeather,
        }
    }
}
