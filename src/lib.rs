use dlopen::raw::Library;
use serde::ser::{Serialize, SerializeStruct};
use serde::Serializer;

use crate::utils::{bayer_pattern, image_type, int_to_binning};

struct Device {}

pub struct ROIFormat {
    pub width: i32,
    pub height: i32,
    pub bin: i32,
    pub img_type: i32,
}

#[repr(C)]
pub struct AsiCameraInfo {
    pub name: [u8; 64],  //the name of the camera, you can display this to the UI
    pub camera_id: i32, //this is used to control everything of the camera in other functions.Start from 0.
    pub max_height: i64, //the max height of the camera
    pub max_width: i64, //the max width of the camera
    pub is_color_cam: i32,
    pub bayer_pattern: i32,
    pub supported_bins: [i32; 16], //1 means bin1 which is supported by every camera, 2 means bin 2 etc.. 0 is the end of supported binning method
    pub supported_video_format: [i32; 8], //this array will content with the support output format type.IMG_END is the end of supported video format
    pub pixel_size: f64, //the pixel size of the camera, unit is um. such like 5.6um
    pub mechanical_shutter: i32,
    pub st4_port: i32,
    pub is_cooler_cam: i32,
    pub is_usb3_host: i32,
    pub is_usb3_camera: i32,
    pub elec_per_adu: f32,
    pub bit_depth: i32,
    pub is_trigger_cam: i32,
    pub unused: [u8; 16],
}

impl Serialize for AsiCameraInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        fn int_to_bool(n: &i32) -> bool {
            match n {
                0 => return false,
                1 => return true,
                _ => panic!("Not a boolean"),
            }
        }

        let mut state = serializer.serialize_struct("AsiCameraInfo", 18)?;
        let mut binning = Vec::with_capacity(16);
        let mut name: Vec<u8> = Vec::with_capacity(64);
        let mut unused: Vec<u8> = Vec::with_capacity(16);
        let mut video_format: Vec<&'static str> = Vec::with_capacity(8);

        // convert the binning array to an array of binning values
        for el in self.supported_bins {
            if el == 0 {
                break;
            } else {
                binning.push(int_to_binning(&el));
            }
        }

        // format the name dropping 0 from the name array
        for el in self.name {
            if el == 0 {
                break;
            } else {
                name.push(el);
            }
        }

        // format the unused dropping 0 from the unused array
        for el in self.unused {
            if el == 0 {
                break;
            } else {
                unused.push(el);
            }
        }

        // format the unused dropping 0 from the unused array
        for el in self.supported_video_format {
            if el == -1 {
                break;
            } else {
                video_format.push(image_type(el));
            }
        }

        state.serialize_field("name", std::str::from_utf8(&name).unwrap())?;
        state.serialize_field("camera_id", &self.camera_id)?;
        state.serialize_field("max_height", &self.max_height)?;
        state.serialize_field("max_width", &self.max_width)?;
        state.serialize_field("is_color_cam", &int_to_bool(&self.is_color_cam))?;
        state.serialize_field("bayer_pattern", &bayer_pattern(&self.bayer_pattern))?;
        state.serialize_field("supported_bins", &binning)?;
        state.serialize_field("supported_video_format", &video_format)?;
        state.serialize_field("pixel_size", &self.pixel_size)?;
        state.serialize_field("mechanical_shutter", &int_to_bool(&self.mechanical_shutter))?;
        state.serialize_field("st4_port", &int_to_bool(&self.st4_port))?;
        state.serialize_field("is_usb3_host", &int_to_bool(&self.is_usb3_host))?;
        state.serialize_field("is_usb3_camera", &int_to_bool(&self.is_usb3_camera))?;
        state.serialize_field("elec_per_adu", &self.elec_per_adu)?;
        state.serialize_field("bit_depth", &self.bit_depth)?;
        state.serialize_field("is_trigger_camera", &int_to_bool(&self.is_trigger_cam))?;
        state.serialize_field("unused", std::str::from_utf8(&unused).unwrap())?;
        state.end()
    }
}

struct ZWOCCDManager {
    name: &'static str,
    shared_object: Library,
    devices: Vec<Device>,
    devices_discovered: i32,
}

pub mod utils {
    use log::error;

    pub fn int_to_binning(n: &i32) -> String {
        return format!("{}x{}", n, n);
    }

    pub fn image_type(n: i32) -> &'static str {
        match n {
            0 => return "RAW8",
            1 => return "RGB24",
            2 => return "RAW16",
            3 => return "Y8",
            -1 => return "END",
            _ => panic!("Image type not supported"),
        }
    }

    pub fn bayer_pattern(n: &i32) -> &'static str {
        match n {
            0 => return "RG",
            1 => return "BG",
            2 => return "GR",
            3 => return "GB",
            _ => panic!("Bayer pattern not recognized"),
        }
    }

    pub fn check_error_code(code: i32) {
        match code {
            0 => (),                                       //ASI_SUCCESS
            1 => error!("ASI_ERROR_INVALID_INDEX"), //no camera connected or index value out of boundary
            2 => error!("ASI_ERROR_INVALID_ID"),    //invalid ID
            3 => error!("ASI_ERROR_INVALID_CONTROL_TYPE"), //invalid control type
            4 => error!("ASI_ERROR_CAMERA_CLOSED"), //camera didn't open
            5 => error!("ASI_ERROR_CAMERA_REMOVED"), //failed to find the camera, maybe the camera has been removed
            6 => error!("ASI_ERROR_INVALID_PATH"),   //cannot find the path of the file
            7 => error!("ASI_ERROR_INVALID_FILEFORMAT"),
            8 => error!("ASI_ERROR_INVALID_SIZE"), //wrong video format size
            9 => error!("ASI_ERROR_INVALID_IMGTYPE"), //unsupported image formate
            10 => error!("ASI_ERROR_OUTOF_BOUNDARY"), //the startpos is out of boundary
            11 => error!("ASI_ERROR_TIMEOUT"),     //timeout
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

pub mod controls {
    use serde::ser::{Serialize, SerializeStruct};
    use serde::Serializer;

    #[repr(C)]
    pub struct AsiControlCaps {
        pub name: [u8; 64],         //the name of the Control like Exposure, Gain etc..
        pub description: [u8; 128], //description of this control
        pub max_value: i64,
        pub min_value: i64,
        pub default_value: i64,
        pub is_auto_supported: i32, //support auto set 1, don't support 0
        pub is_writable: i32,       //some control like temperature can only be read by some cameras
        pub control_type: i32,      //this is used to get value and set value of the control
        pub unused: [u8; 32],
    }

    impl Serialize for AsiControlCaps {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            fn int_to_bool(n: &i32) -> bool {
                match n {
                    0 => return false,
                    1 => return true,
                    _ => panic!("Not a boolean"),
                }
            }

            let mut state = serializer.serialize_struct("AsiControlCaps", 3)?;
            let mut name: Vec<u8> = Vec::with_capacity(64);
            let mut unused: Vec<u8> = Vec::with_capacity(32);
            let mut description: Vec<u8> = Vec::with_capacity(128);

            // format the name dropping 0 from the name array
            for el in self.name {
                if el == 0 {
                    break;
                } else {
                    name.push(el);
                }
            }

            // format the unused dropping 0 from the unused array
            for el in self.unused {
                if el == 0 {
                    break;
                } else {
                    unused.push(el);
                }
            }

            // format the unused dropping 0 from the unused array
            for el in self.description {
                if el == 0 {
                    break;
                } else {
                    description.push(el);
                }
            }

            state.serialize_field("name", std::str::from_utf8(&name).unwrap())?;
            state.serialize_field("max_value", &self.max_value)?;
            state.serialize_field("min_value", &self.min_value)?;
            state.serialize_field("default_value", &self.default_value)?;
            state.serialize_field("is_auto_supported", &int_to_bool(&self.is_auto_supported))?;
            state.serialize_field("is_writable", &int_to_bool(&self.is_writable))?;
            state.serialize_field("control_type", &self.control_type)?;
            state.serialize_field("description", std::str::from_utf8(&description).unwrap())?;
            state.serialize_field("unused", std::str::from_utf8(&unused).unwrap())?;
            state.end()
        }
    }
}

trait Manager {
    fn new(&self, name: &'static str) -> Self;

    // Look on the system for devices and return an integer
    // containing the number of devices found
    fn look_for_devices(&self) -> i32;

    fn get_devices_properties(&self);

    fn init_provider(&self);

    fn pointer_to_vault(&self) -> AsiCameraInfo;
}

impl Manager for ZWOCCDManager {
    fn new(&self, name: &'static str) -> ZWOCCDManager {
        let lib = match Library::open("libASICamera2.so") {
            Ok(so) => so,
            Err(e) => panic!("{}", e),
        };
        let discovered_devices = self.look_for_devices();
        let devices = usize::try_from(discovered_devices).unwrap();

        ZWOCCDManager {
            name: name,
            shared_object: lib,
            devices: Vec::with_capacity(devices),
            devices_discovered: discovered_devices,
        }
    }

    fn pointer_to_vault(&self) -> AsiCameraInfo {
        return AsiCameraInfo {
            name: [0; 64],
            camera_id: 9, //this is used to control everything of the camera in other functions.Start from 0.
            max_height: 0, //the max height of the camera
            max_width: 0, //the max width of the camera

            is_color_cam: 1,
            bayer_pattern: 1,

            supported_bins: [5; 16], //1 means bin1 which is supported by every camera, 2 means bin 2 etc.. 0 is the end of supported binning method
            supported_video_format: [0; 8], //this array will content with the support output format type.IMG_END is the end of supported video format

            pixel_size: 0.0, //the pixel size of the camera, unit is um. such like 5.6um
            mechanical_shutter: 1,
            st4_port: 1,
            is_cooler_cam: 1,
            is_usb3_host: 1,
            is_usb3_camera: 1,
            elec_per_adu: 0.0,
            bit_depth: 0,
            is_trigger_cam: 1,

            unused: [0; 16],
        };
    }

    fn look_for_devices(&self) -> i32 {
        let look_for_devices: extern "C" fn() -> i32 =
            unsafe { self.shared_object.symbol("ASIGetNumOfConnectedCameras") }.unwrap();
        return look_for_devices();
    }

    fn get_devices_properties(&self) {
        let read_device_properties: extern "C" fn(*mut AsiCameraInfo, i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIGetCameraProperty") }.unwrap();

        for index in 0..self.devices_discovered {
            let mut vault = self.pointer_to_vault();
            match read_device_properties(&mut vault, index) {
                0 => println!("Properties retrieved correctly"),
                e => panic!("Error happened: {}", e),
            }
        }
    }

    fn init_provider(&self) {
        let open_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIOpenCamera") }.unwrap();
        let init_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIInitCamera") }.unwrap();
        for index in 0..self.devices_discovered {
            open_camera(index);
            init_camera(index);
        }
    }
}
