use crate::ccd::utils::structs::{AsiCameraInfo, AsiControlCaps, ROIFormat};
use convert_case::{Case, Casing};
use dlopen::raw::Library;
use lightspeed_astro::devices::actions::DeviceActions;
use lightspeed_astro::props::Property;
use log::{debug, error, info};
use rfitsio::fill_to_2880;
use std::{thread, time};
use uuid::Uuid;

pub mod utils {
    use dlopen::raw::Library;
    use lightspeed_astro::props::{Permission, Property};
    use log::{debug, error};

    pub mod capturing {
        use crate::ccd::AstroDevice;
        use crate::ccd::ROIFormat;
        use crate::CcdDevice;
        use asi_rs::utils;
        use dlopen::raw::Library;
        use log::{debug, error, info};
        use rfitsio::fill_to_2880;
        use std::sync::{Arc, RwLock};
        use std::time::Duration;
        use std::time::SystemTime;

        pub fn expose(
            camera_index: i32,
            length: f32,
            width: i32,
            height: i32,
            img_type: i32,
            device: Arc<RwLock<CcdDevice>>,
        ) {
            let lib = match Library::open("libASICamera2.so") {
		Ok(so) => so,
		Err(_) => panic!(
                    "Couldn't find `libASICamera2.so` on the system, please make sure it is installed"
		),
            };
            let start_exposure: extern "C" fn(camera_id: i32) -> i32 =
                unsafe { lib.symbol("ASIStartExposure") }.unwrap();

            let stop_exposure: extern "C" fn(camera_id: i32) -> i32 =
                unsafe { lib.symbol("ASIStopExposure") }.unwrap();

            let exposure_status: extern "C" fn(camera_id: i32, p_status: &mut i32) -> i32 =
                unsafe { lib.symbol("ASIGetExpStatus") }.unwrap();

            let get_data: extern "C" fn(camera_id: i32, &mut [u8], buf_size: i64) -> i32 =
                unsafe { lib.symbol("ASIGetDataAfterExp") }.unwrap();

            info!("Actual width requested: {}", width);
            info!("Actual height requested: {}", height);

            // Create the right sized buffer for the image to be stored.
            // if we shoot at 8 bit it is just width * height
            let mut buffer_size: i32 = width * height;

            buffer_size = match img_type {
                1 | 2 => buffer_size * 2,
                _ => buffer_size,
            };

            let secs_to_micros = length * num::pow(10i32, 6) as f32;
            info!("mu secs {}", secs_to_micros);
            let duration = Duration::from_micros(secs_to_micros as u64);
            let mut image_buffer = Vec::with_capacity(buffer_size as usize);
            unsafe {
                image_buffer.set_len(buffer_size as usize);
            }
            let mut status = 5;

            // Swapping exposure related properties AKA prepare props to show
            // informations about ongoing exposure
            {
                let mut d = device.write().unwrap();
                d.update_internal_property("exposure_status", "EXPOSING");
            }

            utils::check_error_code(start_exposure(camera_index));
            let start = SystemTime::now();

            while start.elapsed().unwrap().as_micros() < duration.as_micros() {
                debug!("Elapsed: {}", start.elapsed().unwrap().as_micros());
                debug!("Duration: {}", duration.as_micros());
                utils::check_error_code(exposure_status(camera_index, &mut status));
                debug!("Status while exposing: {}", status);
                match status {
                    1 | 2 => (),
                    n => error!("An error happened, the exposure status is {}", n),
                }

                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            utils::check_error_code(stop_exposure(camera_index));
            utils::check_error_code(exposure_status(camera_index, &mut status));
            info!("Status after exposure: {}", status);

            match status {
                2 => {
                    {
                        let mut d = device.write().unwrap();
                        d.update_internal_property("exposure_status", "SUCCESS");
                    }

                    utils::check_error_code(get_data(
                        camera_index,
                        &mut image_buffer,
                        buffer_size.into(),
                    ));
                }
                _ => error!("Exposure failed"),
            }

            let mut final_image: Vec<u8> = Vec::new();
            for b in
                b"SIMPLE  =                    T / file conforms to FITS standard                 "
                    .into_iter()
            {
                final_image.push(*b);
            }

            let bitpix = match img_type {
		1 | 2 => format!(
                    "BITPIX  =                   {} / number of bits per data pixel                  ",
                    "16"
		),
		_ => format!(
                    "BITPIX  =                   {} / number of bits per data pixel                  ",
                    " 8"
		),
            };

            let mut naxis1 = String::new();
            if width < 1000 {
                naxis1 = format!(
                    "NAXIS1  =                 {}{} / length of data axis 1                          ",
                    " ", width
		);
            } else {
                naxis1 = format!(
                    "NAXIS1  =                 {} / length of data axis 1                          ",
                    width
		);
            }

            let mut naxis2 = String::new();
            if height < 1000 {
                naxis2 = format!(
                    "NAXIS2  =                 {}{} / length of data axis 2                          ",
                    " ", height
		);
            } else {
                naxis2 = format!(
                    "NAXIS2  =                 {} / length of data axis 2                          ",
                    height
		);
            }

            debug!("Len of NAXIS1 {}", naxis1.len());
            debug!("Len of NAXIS2 {}", naxis2.len());

            for b in bitpix.as_bytes().into_iter() {
                final_image.push(*b);
            }
            for b in
                b"NAXIS   =                    2 / number of axis                                 "
                    .into_iter()
            {
                final_image.push(*b);
            }
            for b in naxis1.as_bytes().into_iter() {
                final_image.push(*b);
            }
            for b in naxis2.as_bytes().into_iter() {
                final_image.push(*b);
            }

            for b in b"END".into_iter() {
                final_image.push(*b);
            }

            debug!("File len after headers: {}", final_image.len());

            for _ in 0..fill_to_2880(final_image.len() as i32) {
                final_image.push(32);
            }

            debug!("File len after filling: {}", final_image.len());

            for b in &image_buffer {
                final_image.push(*b);
            }

            debug!("File len after image: {}", final_image.len());

            for _ in 0..fill_to_2880(final_image.len() as i32) {
                final_image.push(32);
            }

            debug!("File len after filling image: {}", final_image.len());

            match std::fs::write("zwo001.fits", &final_image) {
                Ok(_) => debug!("FITS file saved correctly"),
                Err(e) => error!("FITS file not saved on disk: {}", e),
            };
        }
    }

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

    pub fn new_asi_info() -> crate::ccd::utils::structs::AsiCameraInfo {
        crate::ccd::utils::structs::AsiCameraInfo {
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

    pub fn new_asi_controls_caps() -> crate::ccd::utils::structs::AsiControlCaps {
        crate::ccd::utils::structs::AsiControlCaps {
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

    pub fn new_read_write_prop(name: &str, value: &str, kind: &str) -> Property {
        Property {
            name: name.to_string(),
            value: value.to_string(),
            kind: kind.to_string(),
            permission: Permission::ReadWrite as i32,
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

    /// Given an integer returns a human friendly representation of the image_type
    pub fn int_to_image_type(i: i32) -> String {
        let s = match i {
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

        s.to_string()
    }

    /// Given an array of integers returns a human readable representation of the image type
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

    /// Method used internally by the driver itself to change values for properties that
    /// be manipulated by the user (like the exposure ones)
    fn update_internal_property(&mut self, prop_name: &str, val: &str)
        -> Result<(), DeviceActions>;

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
    fn get_control_caps(&mut self);
    fn get_control_value(&self, cap: &AsiProperty) -> i64;
    fn get_num_of_controls(&self) -> i32;
    fn init_camera_props(&mut self);
    fn asi_caps_to_lightspeed_props(&self) -> Vec<Property>;
    fn fetch_roi_format(&mut self);
    fn get_actual_width(&self) -> i32;
    fn get_actual_height(&self) -> i32;
    fn get_actual_bin(&self) -> String;
    fn get_actual_raw_bin(&self) -> i32;
    fn get_actual_img_type(&self) -> String;
    fn get_actual_raw_img_type(&self) -> i32;
    fn get_index(&self) -> i32;
}

#[derive(Debug, Clone)]
pub struct AsiProperty {
    name: String,
    description: String,
    max_value: i64,
    min_value: i64,
    default_value: i64,
    is_auto_supported: bool,
    is_writable: bool,
    control_type: i32,
}

pub struct CcdDevice {
    id: Uuid,
    name: String,
    pub properties: Vec<Property>,
    library: Library,
    index: i32,
    num_of_controls: i32,
    caps: Vec<AsiProperty>,
    roi: ROIFormat,
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
            index,
            num_of_controls: 0,
            caps: Vec::new(),
            roi: ROIFormat {
                width: 0,
                height: 0,
                bin: 0,
                img_type: 0,
            },
        };
        device.init_camera();
        device.init_camera_props();
        device
    }

    fn fetch_props(&mut self) {
        info!("Fetching properties for device {}", self.name);
    }

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
    fn update_internal_property(
        &mut self,
        prop_name: &str,
        val: &str,
    ) -> Result<(), DeviceActions> {
        info!(
            "driver updating internal property {} with {}",
            prop_name, val
        );
        if let Some(prop_idx) = self.find_property_index(prop_name) {
            let mut prop = self.properties.get_mut(prop_idx).unwrap();
            prop.value = val.to_string();
            Ok(())
        } else {
            Err(DeviceActions::UnknownProperty)
        }
    }

    fn update_property(&mut self, prop_name: &str, val: &str) -> Result<(), DeviceActions> {
        todo!();
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
    fn find_property_index(&self, prop_name: &str) -> Option<usize> {
        let mut index = 256;

        for (idx, prop) in self.properties.iter().enumerate() {
            if prop.name == prop_name {
                index = idx;
                break;
            }
        }
        if index == 256 {
            None
        } else {
            Some(index)
        }
    }
}

impl AsiCcd for CcdDevice {
    fn get_index(&self) -> i32 {
        self.index
    }
    fn init_camera(&mut self) {
        let open_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.library.symbol("ASIOpenCamera") }.unwrap();
        let init_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.library.symbol("ASIInitCamera") }.unwrap();
        debug!("Saying welcome to camera `{}`", self.name);
        utils::check_error_code(open_camera(self.index));
        utils::check_error_code(init_camera(self.index));

        // Check how many capabilities this camera has, reallocate the vector
        // after the number is known
        self.num_of_controls = self.get_num_of_controls();
        self.caps = Vec::with_capacity(self.num_of_controls as usize);

        // Populate now the caps props as they won't change never during the camera's lifetime
        self.get_control_caps();

        // Set the ROI
        self.fetch_roi_format();
    }

    fn get_actual_raw_bin(&self) -> i32 {
        self.roi.bin
    }

    fn get_actual_raw_img_type(&self) -> i32 {
        self.roi.img_type
    }

    fn asi_caps_to_lightspeed_props(&self) -> Vec<Property> {
        let mut props: Vec<Property> = Vec::new();

        for cap in &self.caps {
            info!("CAP name: {}", &cap.name);
            let mut cap_value = self.get_control_value(cap).to_string();
            let mut kind_value = String::from("integer");

            if cap.name == "temperature" {
                let tmp_value = cap_value.parse::<f32>().unwrap() / 10.0;
                cap_value = tmp_value.to_string();
                kind_value = String::from("float")
            }

            // here we create lightspeed properties from AsiCaps
            let prop = Property {
                name: cap.name.to_owned(),
                value: cap_value,
                kind: kind_value,
                permission: cap.is_writable as i32,
            };
            props.push(prop);
        }
        props
    }

    fn get_control_value(&self, cap: &AsiProperty) -> i64 {
        let get_ctrl_val: extern "C" fn(
            camera_id: i32,
            control_type: i32,
            value: &mut i64,
            is_auto_set: &mut i32,
        ) -> i32 = unsafe { self.library.symbol("ASIGetControlValue") }.unwrap();
        debug!("Getting value for prop {}", cap.name);
        let mut is_auto_set = 0;
        let mut val: i64 = 0;
        utils::check_error_code(get_ctrl_val(
            self.index,
            cap.control_type,
            &mut val,
            &mut is_auto_set,
        ));
        debug!(
            "Value for {} is {} - Auto adjusted? {}",
            cap.name, val, cap.is_writable
        );
        val
    }

    fn close(&self) {
        let close_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.library.symbol("ASICloseCamera") }.unwrap();
        debug!("Closing camera {}", self.name);
        close_camera(self.index);
        drop(&self.library);
    }

    fn get_control_caps(&mut self) {
        let get_contr_caps: extern "C" fn(
            camera_id: i32,
            index: i32,
            noc: *mut AsiControlCaps,
        ) -> i32 = unsafe { self.library.symbol("ASIGetControlCaps") }.unwrap();

        for i in 0..self.num_of_controls {
            let mut control_caps = utils::new_asi_controls_caps();
            utils::check_error_code(get_contr_caps(self.index, i, &mut control_caps));
            let cap = AsiProperty {
                name: utils::asi_name_to_string(&control_caps.name).to_case(Case::Snake),
                description: utils::asi_name_to_string(&control_caps.description),
                max_value: control_caps.max_value,
                min_value: control_caps.min_value,
                default_value: control_caps.default_value,
                is_auto_supported: control_caps.is_auto_supported != 0,
                is_writable: control_caps.is_writable != 0,
                control_type: control_caps.control_type,
            };
            info!("Discovered capacity: {:?}", &cap.name);
            self.caps.push(cap);
        }
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

    fn fetch_roi_props(&self) -> Vec<Property> {
        // ROI data: width, height, bin, img type
        let props = vec![
            utils::new_read_write_prop("width", &self.get_actual_width().to_string(), "integer"),
            utils::new_read_write_prop("height", &self.get_actual_height().to_string(), "integer"),
            utils::new_read_write_prop("bin", &self.get_actual_bin().to_string(), "string"),
            utils::new_read_write_prop(
                "image_type",
                &self.get_actual_img_type().to_string(),
                "string",
            ),
        ];

        props
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

        self.properties.push(utils::new_read_write_prop(
            "bin",
            &self.get_actual_bin().to_string(),
            "string",
        ));

        for prop in self.fetch_roi_props() {
            self.properties.push(prop);
        }

        // Properties to build logic around exposures
        self.properties
            .push(utils::new_read_only_prop("exposing", "false", "boolean"));

        self.properties.push(utils::new_read_only_prop(
            "exposure_status",
            "IDLE",
            "string",
        ));

        self.properties
            .push(utils::new_read_only_prop("blob", "", "bytes"));

        for prop in self.asi_caps_to_lightspeed_props() {
            self.properties.push(prop);
        }
    }

    fn fetch_roi_format(&mut self) {
        let _get_roi_format: extern "C" fn(
            camera_id: i32,
            width: &mut i32,
            width: &mut i32,
            bin: &mut i32,
            img_type: &mut i32,
        ) -> i32 = unsafe { self.library.symbol("ASIGetROIFormat") }.unwrap();

        let mut width = 0;
        let mut height = 0;
        let mut bin = 0;
        let mut img_type = 0;

        utils::check_error_code(_get_roi_format(
            self.index,
            &mut width,
            &mut height,
            &mut bin,
            &mut img_type,
        ));
        info!(
            "ROI format => width: {} | height: {} | bin: {} | img type: {}",
            width, height, bin, img_type
        );

        self.roi.width = width;
        self.roi.height = height;
        self.roi.bin = bin;
        self.roi.img_type = img_type;
    }

    fn get_actual_width(&self) -> i32 {
        self.roi.width
    }
    fn get_actual_height(&self) -> i32 {
        self.roi.height
    }
    fn get_actual_bin(&self) -> String {
        utils::int_to_binning_str(&[self.roi.bin])
    }
    fn get_actual_img_type(&self) -> String {
        utils::int_to_image_type(self.roi.img_type)
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
