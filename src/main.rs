use serde::ser::{Serialize, SerializeStruct};
use serde::Serializer;
use log::{debug, error, log_enabled, Level};


use dlopen::raw::Library;
use std::{thread, time};


fn bayer_pattern(n: &i32) -> &'static str {
    match n {
	0 => return "RG",
	1 => return "BG",
	2 => return "GR",
	3 => return "GB",
	_ => panic!("Bayer pattern not recognized")
    }
}

fn image_type(n: i32) -> &'static str {
    match n {
	0 => return "RAW8",
	1 => return "RGB24",
	2 => return "RAW16",
	3 => return "Y8",
	-1 => return "END",
	_ => panic!("Image type not supported")
    }
}

#[repr(C)]
struct AsiCameraInfo {
    //#[serde(with = "BigArray")]
    name: [u8; 64],  //the name of the camera, you can display this to the UI
    camera_id: i32, //this is used to control everything of the camera in other functions.Start from 0.
    max_height: i64, //the max height of the camera
    max_width: i64, //the max width of the camera
    is_color_cam: i32,
    bayer_pattern: i32,
    supported_bins: [i32; 16], //1 means bin1 which is supported by every camera, 2 means bin 2 etc.. 0 is the end of supported binning method
    supported_video_format: [i32; 8], //this array will content with the support output format type.IMG_END is the end of supported video format
    pixel_size: f64, //the pixel size of the camera, unit is um. such like 5.6um
    mechanical_shutter: i32,
    st4_port: i32,
    is_cooler_cam: i32,
    is_usb3_host: i32,
    is_usb3_camera: i32,
    elec_per_adu: f32,
    bit_depth: i32,
    is_trigger_cam: i32,
    unused: [u8; 16],
}

fn int_to_binning(n: &i32) -> String {
    return format!("{}x{}", n, n);
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
		_ => panic!("Not a boolean")
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


fn check_error_code(code: i32) {
    match code {
        0 => (),                                       //ASI_SUCCESS
        1 => panic!("ASI_ERROR_INVALID_INDEX"), //no camera connected or index value out of boundary
        2 => panic!("ASI_ERROR_INVALID_ID"),    //invalid ID
        3 => panic!("ASI_ERROR_INVALID_CONTROL_TYPE"), //invalid control type
        4 => panic!("ASI_ERROR_CAMERA_CLOSED"), //camera didn't open
        5 => panic!("ASI_ERROR_CAMERA_REMOVED"), //failed to find the camera, maybe the camera has been removed
        6 => panic!("ASI_ERROR_INVALID_PATH"),   //cannot find the path of the file
        7 => panic!("ASI_ERROR_INVALID_FILEFORMAT"),
        8 => panic!("ASI_ERROR_INVALID_SIZE"), //wrong video format size
        9 => panic!("ASI_ERROR_INVALID_IMGTYPE"), //unsupported image formate
        10 => panic!("ASI_ERROR_OUTOF_BOUNDARY"), //the startpos is out of boundary
        11 => panic!("ASI_ERROR_TIMEOUT"),     //timeout
        12 => panic!("ASI_ERROR_INVALID_SEQUENCE"), //stop capture first
        13 => panic!("ASI_ERROR_BUFFER_TOO_SMALL"), //buffer size is not big enough
        14 => panic!("ASI_ERROR_VIDEO_MODE_ACTIVE"),
        15 => panic!("ASI_ERROR_EXPOSURE_IN_PROGRESS"),
        16 => panic!("ASI_ERROR_GENERAL_ERROR"), //general error, eg: value is out of valid range
        17 => panic!("ASI_ERROR_INVALID_MODE"),  //the current mode is wrong
        18 => panic!("ASI_ERROR_END"),
        e => panic!("unknown error {}", e),
    }
}


fn main() {
    env_logger::init();
    let lib = match Library::open("x64/libASICamera2.so.1.22") {
        Ok(so) => so,
        Err(e) => panic!("{}", e),
    };
    let mut info = Box::new(AsiCameraInfo {
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
    });

    let look_for_devices: extern "C" fn() -> i32 =
        unsafe { lib.symbol("ASIGetNumOfConnectedCameras") }.unwrap();

    let read_device_properties: extern "C" fn(*mut AsiCameraInfo, i32) -> i32 =
        unsafe { lib.symbol("ASIGetCameraProperty") }.unwrap();

    let open_camera: extern "C" fn(i32) -> i32 = unsafe { lib.symbol("ASIOpenCamera") }.unwrap();

    let init_camera: extern "C" fn(i32) -> i32 = unsafe { lib.symbol("ASIInitCamera") }.unwrap();

    let close_camera: extern "C" fn(i32) -> i32 = unsafe { lib.symbol("ASICloseCamera") }.unwrap();

    fn expose(camera: i32) {
        let lib = match Library::open("x64/libASICamera2.so.1.22") {
            Ok(so) => so,
            Err(e) => panic!("{}", e),
        };
        let start_exposure: extern "C" fn(camera_id: i32) -> i32 =
            unsafe { lib.symbol("ASIStartExposure") }.unwrap();

        let stop_exposure: extern "C" fn(camera_id: i32) -> i32 =
            unsafe { lib.symbol("ASIStopExposure") }.unwrap();

        let exposure_status: extern "C" fn(camera_id: i32, p_status: &i32) -> i32 =
            unsafe { lib.symbol("ASIGetExpStatus") }.unwrap();

        let get_data: extern "C" fn(camera_id: i32, *mut [u8], buf_size: i64) -> i32 =
            unsafe { lib.symbol("ASIGetDataAfterExp") }.unwrap();

        let ten_millis = time::Duration::from_secs(1);
        let _now = time::Instant::now();
        let mut image_buffer = Box::new([0; 1096 * 1936]);
        let mut status = Box::new(5);
        check_error_code(start_exposure(camera));
        check_error_code(exposure_status(camera, &mut *status));
        debug!("Status: {}", status);
        thread::sleep(ten_millis);

        check_error_code(stop_exposure(camera));
        check_error_code(exposure_status(camera, &mut *status));
        debug!("Status2: {}", status);

        check_error_code(get_data(0, &mut *image_buffer, 1096 * 1936));

        match std::fs::write("zwo001.fits", &*image_buffer) {
	    Ok(_) => debug!("FITS file saved correctly"),
	    Err(e) => error!("FITS file not saved on disk: {}", e)
	};
    }

    let found_devices = look_for_devices();
    debug!("Found {} ZWO Cameras", found_devices); 

    check_error_code(read_device_properties(&mut *info, 0));

    if log_enabled!(Level::Debug) {
	println!("Camera name: {}", std::str::from_utf8(&info.name).unwrap());
	println!("Camera ID: {}", &info.camera_id);
	println!("Max width: {}", &info.max_width);
	println!("Max height: {}", &info.max_height);
	println!("Is color? {}", &info.is_color_cam);
	println!("Bayer pattern: {}", &info.bayer_pattern);
	println!("Supported bins: {:?}", &info.supported_bins);
	println!("Supported video format: {:?}", &info.supported_video_format);
	println!("Pixel size: {}", &info.pixel_size);
	println!("Mechanical shutter: {}", &info.mechanical_shutter);
	println!("ST4 port? {}", &info.st4_port);
	println!("Cooled? {}", &info.is_cooler_cam);
	println!("USB3 host? {}", &info.is_usb3_host);
	println!("USB3 camera? {}", &info.is_usb3_camera);
	println!("e- per ADU: {}", &info.elec_per_adu);
	println!("Bit depth: {}", &info.bit_depth);
	println!("Trigger camera?: {}", &info.is_trigger_cam);
	println!("Unused: {}", std::str::from_utf8(&info.unused).unwrap());
    }
	println!("JSON: {}", serde_json::to_string(&info).unwrap());

    check_error_code(open_camera(0));
    check_error_code(init_camera(0));
    expose(0);
    check_error_code(close_camera(0));

    drop(lib);
}
