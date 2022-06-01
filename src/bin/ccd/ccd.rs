use crate::ccd::utils::structs::AsiCameraInfo;
use dlopen::raw::Library;
use lightspeed_astro::devices::actions::DeviceActions;
use lightspeed_astro::props::Property;
use log::{debug, info};
use uuid::Uuid;

pub mod utils {
    use dlopen::raw::Library;
    use lightspeed_astro::props::{Permission, Property};
    use log::{debug, error};

    pub mod structs {
        // The main structure of the ZWO library, this struct is passed to the C function
        // and will contain READ-ONLY phisycal properties of the camera.
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
        pub struct ROIFormat {
            pub width: i32,
            pub height: i32,
            pub bin: i32,
            pub img_type: i32,
        }

        #[repr(C)]
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

    pub fn new_asi_info() -> crate::ccd::AsiCameraInfo {
        crate::ccd::AsiCameraInfo {
            name: [0; 64],
            camera_id: 9,
            max_height: 0,
            max_width: 0,
            is_color_cam: 1,
            bayer_pattern: 1,
            supported_bins: [5; 16],
            supported_video_format: [0; 8],
            pixel_size: 0.0,
            mechanical_shutter: 1,
            st4_port: 1,
            is_cooler_cam: 1,
            is_usb3_host: 1,
            is_usb3_camera: 1,
            elec_per_adu: 0.0,
            bit_depth: 0,
            is_trigger_cam: 1,
            unused: [0; 16],
        }
    }

    pub fn asi_name_to_string(name_array: &[u8]) -> String {
        let mut index: usize = 0;

        // format the name dropping 0 from the name array
        for (_, el) in name_array.into_iter().enumerate() {
            if *el == 0 {
                break;
            }
            index += 1
        }
        std::str::from_utf8(&name_array[0..index])
            .unwrap()
            .to_string()
    }

    pub fn new_read_only_prop(name: &str, value: &str, kind: &str) -> Property {
        Property {
            name: name.to_string(),
            value: value.to_string(),
            kind: kind.to_string(),
            permission: Permission::ReadOnly as i32,
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

    pub fn bayer_pattern_to_str(n: &i32) -> &'static str {
        match n {
            0 => return "RG",
            1 => return "BG",
            2 => return "GR",
            3 => return "GB",
            _ => {
                error!("Bayer pattern not recognized");
                return "UNKNOWN";
            }
        }
    }

    pub fn look_for_devices() -> i32 {
        let lib = match Library::open("libASICamera2.so") {
            Ok(so) => so,
            Err(_) => panic!(
                "Couldn't find `libASICamera2.so` on the system, please make sure it is installed"
            ),
        };
        let look_for_devices: extern "C" fn() -> i32 =
            unsafe { lib.symbol("ASIGetNumOfConnectedCameras") }.unwrap();
        let num_of_devs = look_for_devices();
        debug!("Found {} ZWO Cameras", num_of_devs);
        num_of_devs
    }

    /// Given an array of int it returns a string containing the
    /// corresponding binning values for those numbers.
    ///
    /// For example if we have an array like [1,2,3] it will return
    /// "1x1,2x2,3x3"
    pub fn int_to_binning_str(array: &[i32]) -> String {
        // Prepare the string, it must be long 4 * array.len() -1
        // as every number will be represented as NxN,
        let array_length = array.len();
        let mut representation = String::with_capacity(4 * array_length - 1);

        for (index, el) in array.iter().enumerate() {
            if *el == 0 {
                break;
            }

            if index != 0 {
                representation.push(',');
            }

            let s = format!("{}x{}", el, el);
            representation.push_str(&s)
        }

        representation
    }

    /// Given an int returns a human readable representation of the image type
    pub fn int_to_image_type_array(array: &[i32]) -> String {
        let mut representation = String::new();

        for (index, el) in array.iter().enumerate() {
            if *el == -1 {
                break;
            }

            if index != 0 {
                representation.push(',');
            }

            let s = match el {
                0 => "RAW8",
                1 => "RGB24",
                2 => "RAW16",
                3 => "Y8",
                -1 => "END",
                i => {
                    error!("Image type `{}` not supported", i);
                    "UNKNOWN"
                }
            };

            representation.push_str(&s)
        }

        representation
    }
}
pub trait AstroDevice {
    /// Main and only entrypoint to create a new serial device.
    ///
    /// A device that doesn't work/cannot communicate with is not really useful
    /// so this may return `None` if there is something wrong with the just
    /// discovered device.
    fn new(index: i32) -> Self
    where
        Self: Sized;

    /// Use this method to fetch the real properties from the device,
    /// this should not be called directly from clients ideally,
    /// for that goal `get_properties` should be used.
    fn fetch_props(&mut self);

    /// Use this method to return the id of the device as a uuid.
    fn get_id(&self) -> Uuid;

    /// Use this method to return the name of the device (e.g. ZWO533MC).
    fn get_name(&self) -> &String;

    /// Use this method to return the actual cached state stored into `self.properties`.
    fn get_properties(&self) -> &Vec<Property>;

    /// Method to be used when receving requests from clients to update properties.
    ///
    /// Ideally this should call internally `update_property_remote` which will be
    /// responsible to trigger the action against the device to update the property
    /// on the device itself, if the action is successful the last thing this method
    /// does would be to update the property inside `self.properties`.
    fn update_property(&mut self, prop_name: &str, val: &str) -> Result<(), DeviceActions>;

    /// Use this method to send a command to the device to change the requested property.
    ///
    /// Ideally this method will be a big `match` clause where the matching will execute
    /// `self.send_command` to issue a serial command to the device.
    fn update_property_remote(&mut self, prop_name: &str, val: &str) -> Result<(), DeviceActions>;

    /// Properties are packed into a vector so to find them we need to
    /// lookup the index, use this method to do so.
    fn find_property_index(&self, prop_name: &str) -> Option<usize>;
}

pub trait AsiCcd {
    fn init_camera(&mut self);
    fn close(&self);
    fn get_control_caps(&self, camera_id: i32, num_of_controls: i32);
    fn get_num_of_controls(&self) -> i32;
    fn expose(&self, camera_id: i32, length: f32) -> Vec<u8>;
    fn init_camera_props(&mut self);
}

pub struct CcdDevice {
    id: Uuid,
    name: String,
    pub properties: Vec<Property>,
    library: Library,
    index: i32,
    num_of_controls: i32,
}

impl AstroDevice for CcdDevice {
    fn new(index: i32) -> Self
    where
        Self: Sized,
    {
        let lib = match Library::open("libASICamera2.so") {
            Ok(so) => so,
            Err(_) => panic!(
                "Couldn't find `libASICamera2.so` on the system, please make sure it is installed"
            ),
        };
        let mut device = CcdDevice {
            id: Uuid::new_v4(),
            name: "".to_string(),
            properties: Vec::new(),
            library: lib,
            index: index,
            num_of_controls: 0,
        };
        device.init_camera_props();
        device.init_camera();
        device
    }

    fn fetch_props(&mut self) {}

    /// Use this method to return the id of the device as a uuid.
    fn get_id(&self) -> Uuid {
        self.id
    }

    /// Use this method to return the name of the device (e.g. ZWO533MC).
    fn get_name(&self) -> &String {
        &self.name
    }

    /// Use this method to return the actual cached state stored into `self.properties`.
    fn get_properties(&self) -> &Vec<Property> {
        &self.properties
    }

    /// Method to be used when receving requests from clients to update properties.
    ///
    /// Ideally this should call internally `update_property_remote` which will be
    /// responsible to trigger the action against the device to update the property
    /// on the device itself, if the action is successful the last thing this method
    /// does would be to update the property inside `self.properties`.
    fn update_property(&mut self, _prop_name: &str, _val: &str) -> Result<(), DeviceActions> {
        todo!()
    }

    /// Use this method to send a command to the device to change the requested property.
    ///
    /// Ideally this method will be a big `match` clause where the matching will execute
    /// `self.send_command` to issue a serial command to the device.
    fn update_property_remote(
        &mut self,
        _prop_name: &str,
        _val: &str,
    ) -> Result<(), DeviceActions> {
        todo!()
    }

    /// Properties are packed into a vector so to find them we need to
    /// lookup the index, use this method to do so.
    fn find_property_index(&self, _prop_name: &str) -> Option<usize> {
        todo!()
    }
}

impl AsiCcd for CcdDevice {
    fn init_camera(&mut self) {
        let open_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.library.symbol("ASIOpenCamera") }.unwrap();
        let init_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.library.symbol("ASIInitCamera") }.unwrap();
        debug!("Saying welcome to camera `{}`", self.name);
        utils::check_error_code(open_camera(self.index));
        utils::check_error_code(init_camera(self.index));
        self.num_of_controls = self.get_num_of_controls();
    }

    fn close(&self) {
        let close_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.library.symbol("ASICloseCamera") }.unwrap();
        debug!("Closing camera {}", self.name);
        close_camera(self.index);
        drop(&self.library);
    }

    fn get_control_caps(&self, _camera_id: i32, _num_of_controls: i32) {
        todo!();
    }

    fn get_num_of_controls(&self) -> i32 {
        let get_num_of_controls: extern "C" fn(camera_id: i32, noc: *mut i32) -> i32 =
            unsafe { self.library.symbol("ASIGetNumOfControls") }.unwrap();
        let mut num_of_controls = 0;
        let result = get_num_of_controls(self.index, &mut num_of_controls);
        utils::check_error_code(result);
        info!(
            "Found: {} controls for camera {}",
            num_of_controls, self.name
        );
        num_of_controls
    }

    fn expose(&self, _camera_id: i32, _length: f32) -> Vec<u8> {
        todo!();
    }

    fn init_camera_props(&mut self) {
        let read_device_properties: extern "C" fn(*mut AsiCameraInfo, i32) -> i32 =
            unsafe { self.library.symbol("ASIGetCameraProperty") }.unwrap();

        let mut info = utils::new_asi_info();
        utils::check_error_code(read_device_properties(&mut info, self.index));

        // Name the device now
        self.name = utils::asi_name_to_string(&info.name);
        self.index = info.camera_id;

        // 16 properties from AsiCameraInfo - unused and name are ignored

        debug!("ADDING CAMERA PROPERTIES");
        self.properties.push(utils::new_read_only_prop(
            "camera_id",
            &info.camera_id.to_string(),
            "integer",
        ));
        self.properties.push(utils::new_read_only_prop(
            "max_height",
            &info.max_height.to_string(),
            "integer",
        ));
        self.properties.push(utils::new_read_only_prop(
            "max_width",
            &info.max_width.to_string(),
            "integer",
        ));
        self.properties.push(utils::new_read_only_prop(
            "is_color",
            &info.is_color_cam.to_string(),
            "boolean",
        ));
        self.properties.push(utils::new_read_only_prop(
            "bayer_pattern",
            utils::bayer_pattern_to_str(&info.bayer_pattern),
            "string",
        ));
        self.properties.push(utils::new_read_only_prop(
            "supported_bins",
            &utils::int_to_binning_str(&info.supported_bins),
            "array",
        ));
        self.properties.push(utils::new_read_only_prop(
            "supported_video_formats",
            &utils::int_to_image_type_array(&info.supported_video_format),
            "array",
        ));
        self.properties.push(utils::new_read_only_prop(
            "pixel_size",
            &info.pixel_size.to_string(),
            "float",
        ));
        self.properties.push(utils::new_read_only_prop(
            "has_shutter",
            &info.mechanical_shutter.to_string(),
            "boolean",
        ));
        self.properties.push(utils::new_read_only_prop(
            "st4",
            &info.st4_port.to_string(),
            "boolean",
        ));
        self.properties.push(utils::new_read_only_prop(
            "elec_per_adu",
            &info.elec_per_adu.to_string(),
            "float",
        ));
        self.properties.push(utils::new_read_only_prop(
            "bit_depth",
            &info.bit_depth.to_string(),
            "integer",
        ));
    }
}

#[cfg(test)]
mod test_utils {
    use crate::ccd::utils;

    #[test]
    fn test_binning_array_to_string() {
        let bin_array: [i32; 7] = [1, 2, 3, 4, 0, 0, 0];
        let expected_str: &str = "1x1,2x2,3x3,4x4";
        let result = utils::int_to_binning_str(&bin_array);

        assert_eq!(result, expected_str);
    }

    #[test]
    fn test_asi_name_parsed_correctly() {
        let array_name = [
            0x5a, 0x57, 0x4f, 0x20, 0x53, 0x55, 0x50, 0x45, 0x52, 0x20, 0x44, 0x55, 0x50, 0x45,
            0x52, 0x20, 0x74, 0x75, 0x72, 0x62, 0x6f,
        ];
        let expected_str = "ZWO SUPER DUPER turbo";
        let result = utils::asi_name_to_string(&array_name);

        assert_eq!(result, expected_str);
    }

    #[test]
    fn test_video_format_parsed_correctly() {
        let array_name = [0, 1, 2, -1];
        let expected_str = "RAW8,RGB24,RAW16";
        let result = utils::int_to_image_type_array(&array_name);

        assert_eq!(result, expected_str);
    }
}