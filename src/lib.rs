use dlopen::raw::Library;
use log::{debug, error, info};
use rfitsio::fill_to_2880;
use serde::ser::{Serialize, SerializeStruct};
use serde::Serializer;

use std::{thread, time};

use crate::controls::AsiControlCaps;
use crate::utils::{bayer_pattern, image_type, int_to_binning};

pub struct Property {}

pub struct Device {
    info: AsiCameraInfo,
    //properties: Vec<Property>
}

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

pub struct ZWOCCDManager {
    pub name: &'static str,
    pub shared_object: Library,
    pub devices: Vec<Device>,
    pub devices_discovered: i32,
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

pub trait Manager {
    fn new(name: &'static str) -> Self;

    // Look on the system for devices and return an integer
    // containing the number of devices found
    fn look_for_devices(lib: &Library) -> i32;

    fn get_devices_properties(&self);

    fn init_provider(&self);

    fn get_info_struct(&self) -> AsiCameraInfo;

    fn close(&self);

    fn get_roi_format(&self, camera_id: i32) -> ROIFormat;

    fn set_roi_format(&self, camera_id: i32, width: i32, height: i32, bin: i32, img_type: i32);

    fn expose(&self, camera_id: i32, length: f32);
}

impl ZWOCCDManager {
    pub fn get_num_of_controls(&self, camera_id: i32) -> i32 {
        let get_num_of_controls: extern "C" fn(camera_id: i32, noc: *mut i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIGetNumOfControls") }.unwrap();
        let mut num_of_controls = 0;
        let result = get_num_of_controls(camera_id, &mut num_of_controls);
        utils::check_error_code(result);
        info!(
            "Found: {} controls for camera {}",
            num_of_controls, camera_id
        );
        num_of_controls
    }

    pub fn get_control_caps(&self, camera_id: i32, num_of_controls: i32) {
        let get_contr_caps: extern "C" fn(
            camera_id: i32,
            index: i32,
            noc: *mut AsiControlCaps,
        ) -> i32 = unsafe { self.shared_object.symbol("ASIGetControlCaps") }.unwrap();

        for i in 0..num_of_controls {
            let mut control_caps = AsiControlCaps {
                name: [0; 64],
                description: [0; 128],
                max_value: 0,
                min_value: 0,
                default_value: 0,
                is_auto_supported: 0,
                is_writable: 0,
                control_type: 0,
                unused: [0; 32],
            };
            utils::check_error_code(get_contr_caps(camera_id, i, &mut control_caps));
            info!(
                "Capability: {}",
                serde_json::to_string(&control_caps).unwrap()
            );
        }
    }
}

impl Manager for ZWOCCDManager {
    fn new(name: &'static str) -> ZWOCCDManager {
        let lib = match Library::open("libASICamera2.so") {
            Ok(so) => so,
            Err(e) => panic!("{}", e),
        };
        let discovered_devices = Self::look_for_devices(&lib);
        let devices = usize::try_from(discovered_devices).unwrap();

        ZWOCCDManager {
            name: name,
            shared_object: lib,
            devices: Vec::with_capacity(devices),
            devices_discovered: discovered_devices,
        }
    }

    fn get_info_struct(&self) -> AsiCameraInfo {
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

    fn look_for_devices(lib: &Library) -> i32 {
        let look_for_devices: extern "C" fn() -> i32 =
            unsafe { lib.symbol("ASIGetNumOfConnectedCameras") }.unwrap();
        let num_of_devs = look_for_devices();
        debug!("Found {} ZWO Cameras", num_of_devs);
        num_of_devs
    }

    fn get_devices_properties(&self) {
        let read_device_properties: extern "C" fn(*mut AsiCameraInfo, i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIGetCameraProperty") }.unwrap();

        for index in 0..self.devices_discovered {
            let mut info = self.get_info_struct();
            utils::check_error_code(read_device_properties(&mut info, index));
            let device = Device { info: info };
            debug!("{}", serde_json::to_string_pretty(&device.info).unwrap());
        }
    }

    fn init_provider(&self) {
        let open_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIOpenCamera") }.unwrap();
        let init_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIInitCamera") }.unwrap();
        for index in 0..self.devices_discovered {
            utils::check_error_code(open_camera(index));
            utils::check_error_code(init_camera(index));
        }
    }

    fn close(&self) {
        let close_camera: extern "C" fn(i32) -> i32 =
            unsafe { self.shared_object.symbol("ASICloseCamera") }.unwrap();
        for i in 0..self.devices_discovered {
            debug!("Closing camera {}", i);
            close_camera(i);
        }
        drop(&self.shared_object);
    }

    fn get_roi_format(&self, camera_id: i32) -> ROIFormat {
        let _get_roi_format: extern "C" fn(
            camera_id: i32,
            width: &mut i32,
            width: &mut i32,
            bin: &mut i32,
            img_type: &mut i32,
        ) -> i32 = unsafe { self.shared_object.symbol("ASIGetROIFormat") }.unwrap();

        let mut width = 0;
        let mut height = 0;
        let mut bin = 0;
        let mut img_type = 0;

        utils::check_error_code(_get_roi_format(
            camera_id,
            &mut width,
            &mut height,
            &mut bin,
            &mut img_type,
        ));
        info!(
            "ROI format => width: {} | height: {} | bin: {} | img type: {}",
            width, height, bin, img_type
        );

        ROIFormat {
            width: width,
            height: height,
            bin: bin,
            img_type: img_type,
        }
    }

    fn set_roi_format(&self, camera_id: i32, width: i32, height: i32, bin: i32, img_type: i32) {
        let _set_roi_format: extern "C" fn(
            camera_id: i32,
            width: i32,
            height: i32,
            bin: i32,
            img_type: i32,
        ) -> i32 = unsafe { self.shared_object.symbol("ASISetROIFormat") }.unwrap();

        utils::check_error_code(_set_roi_format(camera_id, width, height, bin, img_type));
    }

    fn expose(&self, camera_id: i32, seconds: f32) {
        let start_exposure: extern "C" fn(camera_id: i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIStartExposure") }.unwrap();

        let stop_exposure: extern "C" fn(camera_id: i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIStopExposure") }.unwrap();

        let exposure_status: extern "C" fn(camera_id: i32, p_status: &mut i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIGetExpStatus") }.unwrap();

        let get_data: extern "C" fn(camera_id: i32, &mut [u8], buf_size: i64) -> i32 =
            unsafe { self.shared_object.symbol("ASIGetDataAfterExp") }.unwrap();

        let format = self.get_roi_format(camera_id);
        debug!("Actual width requested: {}", format.width);
        debug!("Actual height requested: {}", format.height);

        // Create the right sized buffer for the image to be stored.
        // if we shoot at 8 bit it is just width * height
        let mut buffer_size: i32 = format.width * format.height;

        buffer_size = match format.img_type {
            1 | 2 => buffer_size * 2,
            _ => buffer_size,
        };

        let secs_to_micros = seconds * num::pow(10i32, 6) as f32;
        debug!("mu secs {}", secs_to_micros);
        let duration = time::Duration::from_micros(secs_to_micros as u64);
        let mut image_buffer = Vec::with_capacity(buffer_size as usize);
        unsafe {
            image_buffer.set_len(buffer_size as usize);
        }
        let mut status = 5;
        utils::check_error_code(start_exposure(camera_id));
        utils::check_error_code(exposure_status(camera_id, &mut status));
        debug!("Status: {}", status);
        thread::sleep(duration);

        utils::check_error_code(stop_exposure(camera_id));
        utils::check_error_code(exposure_status(camera_id, &mut status));
        debug!("Status2: {}", status);

        utils::check_error_code(get_data(0, &mut image_buffer, buffer_size.into()));

        let mut final_image: Vec<u8> = Vec::new();
        for b in b"SIMPLE  =                    T / file conforms to FITS standard                 "
            .into_iter()
        {
            final_image.push(*b);
        }

        let bitpix = match format.img_type {
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
        if format.width < 1000 {
            naxis1 = format!(
                "NAXIS1  =                 {}{} / length of data axis 1                          ",
                " ", format.width
            );
        } else {
            naxis1 = format!(
                "NAXIS1  =                 {} / length of data axis 1                          ",
                format.width
            );
        }

        let mut naxis2 = String::new();
        if format.height < 1000 {
            naxis2 = format!(
                "NAXIS2  =                 {}{} / length of data axis 2                          ",
                " ", format.height
            );
        } else {
            naxis2 = format!(
                "NAXIS2  =                 {} / length of data axis 2                          ",
                format.height
            );
        }

        debug!("Len of NAXIS1 {}", naxis1.len());
        debug!("Len of NAXIS2 {}", naxis2.len());

        for b in bitpix.as_bytes().into_iter() {
            final_image.push(*b);
        }
        for b in b"NAXIS   =                    2 / number of axis                                 "
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
