use log::{debug, error, info, log_enabled, Level};

use dlopen::raw::Library;
use rfitsio::fill_to_2880;
use std::{thread, time};

use asi_rs::controls::AsiControlCaps;
use asi_rs::utils::check_error_code;
use asi_rs::{AsiCameraInfo, ROIFormat};

fn main() {
    env_logger::init();
    let lib = match Library::open("x64/libASICamera2.so.1.22") {
        Ok(so) => so,
        Err(e) => panic!("{}", e),
    };
    let mut info = AsiCameraInfo {
        name: [0; 64],
        camera_id: 9,
        max_height: 0,
        max_width: 0,
        is_color_cam: 1,
        bayer_pattern: 1,
        supported_bins: [0; 16],
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
    };

    let look_for_devices: extern "C" fn() -> u8 =
        unsafe { lib.symbol("ASIGetNumOfConnectedCameras") }.unwrap();

    let read_device_properties: extern "C" fn(*mut AsiCameraInfo, i32) -> i32 =
        unsafe { lib.symbol("ASIGetCameraProperty") }.unwrap();

    let open_camera: extern "C" fn(i32) -> i32 = unsafe { lib.symbol("ASIOpenCamera") }.unwrap();

    let init_camera: extern "C" fn(i32) -> i32 = unsafe { lib.symbol("ASIInitCamera") }.unwrap();

    let close_camera: extern "C" fn(i32) -> i32 = unsafe { lib.symbol("ASICloseCamera") }.unwrap();

    fn get_roi_format(camera_id: i32) -> ROIFormat {
        let lib = match Library::open("x64/libASICamera2.so.1.22") {
            Ok(so) => so,
            Err(e) => panic!("{}", e),
        };

        let _get_roi_format: extern "C" fn(
            camera_id: i32,
            width: &mut i32,
            width: &mut i32,
            bin: &mut i32,
            img_type: &mut i32,
        ) -> i32 = unsafe { lib.symbol("ASIGetROIFormat") }.unwrap();
        let mut width = 0;
        let mut height = 0;
        let mut bin = 0;
        let mut img_type = 0;

        check_error_code(_get_roi_format(
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

        return ROIFormat {
            width: width,
            height: height,
            bin: bin,
            img_type: img_type,
        };
    }

    fn set_roi_format(camera_id: i32, width: i32, height: i32, bin: i32, img_type: i32) {
        let lib = match Library::open("x64/libASICamera2.so.1.22") {
            Ok(so) => so,
            Err(e) => panic!("{}", e),
        };

        let _set_roi_format: extern "C" fn(
            camera_id: i32,
            width: i32,
            height: i32,
            bin: i32,
            img_type: i32,
        ) -> i32 = unsafe { lib.symbol("ASISetROIFormat") }.unwrap();

        check_error_code(_set_roi_format(camera_id, width, height, bin, img_type));
    }

    fn get_num_of_controls(camera_id: i32) -> i32 {
        let lib = match Library::open("x64/libASICamera2.so.1.22") {
            Ok(so) => so,
            Err(e) => panic!("{}", e),
        };

        let get_num_of_controls: extern "C" fn(camera_id: i32, noc: *mut i32) -> i32 =
            unsafe { lib.symbol("ASIGetNumOfControls") }.unwrap();
        let mut num_of_controls = 0;
        let result = get_num_of_controls(camera_id, &mut num_of_controls);
        check_error_code(result);
        info!("Found: {} controls", num_of_controls);
        return num_of_controls;
    }

    fn get_control_caps(camera_id: i32, num_of_controls: i32) {
        let lib = match Library::open("x64/libASICamera2.so.1.22") {
            Ok(so) => so,
            Err(e) => panic!("{}", e),
        };

        let get_contr_caps: extern "C" fn(
            camera_id: i32,
            index: i32,
            noc: *mut AsiControlCaps,
        ) -> i32 = unsafe { lib.symbol("ASIGetControlCaps") }.unwrap();

        for i in 0..num_of_controls {
            let mut control_caps = Box::new(AsiControlCaps {
                name: [0; 64],
                description: [0; 128],
                max_value: 0,
                min_value: 0,
                default_value: 0,
                is_auto_supported: 0,
                is_writable: 0,
                control_type: 0,
                unused: [0; 32],
            });
            check_error_code(get_contr_caps(camera_id, i, &mut *control_caps));
            info!(
                "Capability: {}",
                serde_json::to_string(&control_caps).unwrap()
            );
        }
    }

    fn expose(camera: i32) {
        let lib = match Library::open("x64/libASICamera2.so.1.22") {
            Ok(so) => so,
            Err(e) => panic!("{}", e),
        };
        let start_exposure: extern "C" fn(camera_id: i32) -> i32 =
            unsafe { lib.symbol("ASIStartExposure") }.unwrap();

        let stop_exposure: extern "C" fn(camera_id: i32) -> i32 =
            unsafe { lib.symbol("ASIStopExposure") }.unwrap();

        let exposure_status: extern "C" fn(camera_id: i32, p_status: &mut i32) -> i32 =
            unsafe { lib.symbol("ASIGetExpStatus") }.unwrap();

        let get_data: extern "C" fn(camera_id: i32, &mut [u8], buf_size: i64) -> i32 =
            unsafe { lib.symbol("ASIGetDataAfterExp") }.unwrap();

        let format = get_roi_format(camera);
        debug!("Actual width requested: {}", format.width);
        debug!("Actual height requested: {}", format.height);

        // Create the right sized buffer for the image to be stored.
        // if we shoot at 8 bit it is just width * height
        let mut buffer_size: i32 = format.width * format.height;

        buffer_size = match format.img_type {
            1 | 2 => buffer_size * 2,
            _ => buffer_size,
        };

        let ten_millis = time::Duration::from_millis(500);
        let _now = time::Instant::now();
        let mut image_buffer = Vec::with_capacity(buffer_size as usize);
        unsafe {
            image_buffer.set_len(buffer_size as usize);
        }
        let mut status = 5;
        check_error_code(start_exposure(camera));
        check_error_code(exposure_status(camera, &mut status));
        debug!("Status: {}", status);
        thread::sleep(ten_millis);

        check_error_code(stop_exposure(camera));
        check_error_code(exposure_status(camera, &mut status));
        debug!("Status2: {}", status);

        check_error_code(get_data(0, &mut image_buffer, buffer_size.into()));

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

    let found_devices = look_for_devices();
    debug!("Found {} ZWO Cameras", found_devices);

    check_error_code(read_device_properties(&mut info, 0));

    if log_enabled!(Level::Debug) {
        debug!("Camera name: {}", std::str::from_utf8(&info.name).unwrap());
        debug!("Camera ID: {}", &info.camera_id);
        debug!("Max width: {}", &info.max_width);
        debug!("Max height: {}", &info.max_height);
        debug!("Is color? {}", &info.is_color_cam);
        debug!("Bayer pattern: {}", &info.bayer_pattern);
        debug!("Supported bins: {:?}", &info.supported_bins);
        debug!("Supported video format: {:?}", &info.supported_video_format);
        debug!("Pixel size: {}", &info.pixel_size);
        debug!("Mechanical shutter: {}", &info.mechanical_shutter);
        debug!("ST4 port? {}", &info.st4_port);
        debug!("Cooled? {}", &info.is_cooler_cam);
        debug!("USB3 host? {}", &info.is_usb3_host);
        debug!("USB3 camera? {}", &info.is_usb3_camera);
        debug!("e- per ADU: {}", &info.elec_per_adu);
        debug!("Bit depth: {}", &info.bit_depth);
        debug!("Trigger camera?: {}", &info.is_trigger_cam);
        debug!("Unused: {}", std::str::from_utf8(&info.unused).unwrap());
    }
    info!(
        "General properties: {}",
        serde_json::to_string_pretty(&info).unwrap()
    );

    check_error_code(open_camera(0));
    check_error_code(init_camera(0));
    let noc = get_num_of_controls(0);
    get_control_caps(0, noc);
    get_roi_format(0);
    set_roi_format(0, 1936, 1096, 1, 0);
    get_roi_format(0);
    expose(0);
    check_error_code(close_camera(0));

    drop(lib);
}
