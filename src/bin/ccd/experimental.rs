use crate::utils::fetch_control_caps;
use crate::utils::get_num_of_controls;
use libasi::camera::AsiCameraInfo;
use lightspeed_astro::devices::actions::DeviceActions;
use lightspeed_astro::properties::{BoolProperty, Permission, Property};
use log::{debug, info};
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

pub mod utils {
    use crate::experimental::AsiProperty;
    use convert_case::{Case, Casing};
    use libasi::camera::AsiControlCaps;
    use log::{error, info, warn};

    pub mod generics {
        use crate::utils::asi_id_to_string;
        use libasi::camera::{get_cam_id, set_cam_id, AsiID};
        use log::{debug, info};
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        pub fn get_camera_id(camera_index: i32) -> String {
            let mut id: AsiID = AsiID::new();
            get_cam_id(camera_index, &mut id);

            // if the AsiID is a bunch of 0, we set a random ID and we dump it to the camera flash
            // memory. If you are wondering why, the reason is the following; one may want to use multiple
            // cameras even of the same type for taking pics, if both are presented with only the ZWO name
            // it may be diffcult to manage both if one disconnects and reconnect, or just to pick one
            // from the UI, setting the ID through ASISetID survives reboot
            if id.id == [0, 0, 0, 0, 0, 0, 0, 0] {
                debug!("Setting a random uid");
                crate::utils::generics::set_camera_id(camera_index, None);
            }
            let id_str = asi_id_to_string(&id.id);
            info!("ASI ID for camera with index {}: {:?}", camera_index, &id);
            id_str
        }

        pub fn set_camera_id(camera_index: i32, cam_id: Option<[u8; 8]>) {
            // int pointer that will be passed to the C function to be filled
            let mut id: AsiID = AsiID::new();

            match cam_id {
                Some(i) => id.id = i,
                None => {
                    let rand_string: String = thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(8)
                        .map(char::from)
                        .collect();
                    let r = rand_string.as_bytes();

                    for (i, byte) in r.iter().enumerate() {
                        id.id[i] = *byte;
                    }
                }
            }

            info!(
                "SET ASI ID for camera with index {}: {:?}",
                camera_index,
                asi_id_to_string(&id.id)
            );
            set_cam_id(camera_index, id);
        }
    }

    pub mod capturing {
        use crate::CcdDevice;
        use libasi::camera::{
            download_exposure, exposure_status, set_control_value, start_exposure,
        };
        use log::{debug, error, info};
        use rfitsio::fill_to_2880;
        use std::sync::{Arc, RwLock};
        use std::time::SystemTime;

        pub fn expose(
            camera_index: i32,
            length: f32,
            width: i32,
            height: i32,
            img_type: i32,
            device: Arc<RwLock<CcdDevice>>,
        ) {
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

            let mut image_buffer = Vec::with_capacity(buffer_size as usize);
            unsafe {
                image_buffer.set_len(buffer_size as usize);
            }
            let mut status = 5;

            // Swapping exposure related properties AKA prepare props to show
            // informations about ongoing exposure
            {
                let mut d = device.write().unwrap();
                d.exposure_status
                    .set_val(std::borrow::Cow::Borrowed("EXPOSING"));
            }

            // Set the value of the exposure on the driver
            #[cfg(unix)]
            {
                set_control_value(
                    camera_index,
                    libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32,
                    secs_to_micros as i64,
                    0,
                );
            }

            #[cfg(windows)]
            {
                set_control_value(
                    camera_index,
                    libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32,
                    secs_to_micros as i32,
                    0,
                );
            }

            // Send the command to start the exposure
            start_exposure(camera_index);

            // Check the status, when exposing it should be 1
            exposure_status(camera_index, &mut status);

            let start = SystemTime::now();

            // Loop until the status change
            while status == 1 {
                exposure_status(camera_index, &mut status);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            info!("Elapsed: {}", start.elapsed().unwrap().as_micros());

            match status {
                2 => info!("Exposure successful"),
                n => error!("An error happened, the exposure status is {}", n),
            }

            info!("Status after exposure: {}", status);

            match status {
                2 => {
                    {
                        let mut d = device.write().unwrap();
                        d.exposure_status
                            .set_val(std::borrow::Cow::Borrowed("SUCCESS"));
                    }

                    download_exposure(camera_index, image_buffer.as_mut_ptr(), buffer_size.into());
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

            let naxis1;
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

            let naxis2;
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

            match std::fs::write(
                format!("zwo-{}-001.fits", &device.read().unwrap().name),
                &final_image,
            ) {
                Ok(_) => debug!("FITS file saved correctly"),
                Err(e) => error!("FITS file not saved on disk: {}", e),
            };
        }
    }

    pub fn asi_name_to_string_i8(name_array: &[i8]) -> String {
        let mut to_u8: Vec<u8> = vec![];

        // format the name dropping 0 from the name array
        for (_, el) in name_array.into_iter().enumerate() {
            if *el == 0 {
                break;
            }
            match (*el).try_into() {
                Ok(v) => to_u8.push(v),
                Err(_) => to_u8.push(0x23),
            }
        }
        if let Ok(id) = std::str::from_utf8(&to_u8) {
            id.to_string()
        } else {
            String::from("UNKNOWN")
        }
    }

    pub fn asi_id_to_string(id_array: &[u8]) -> String {
        let mut index: usize = 0;

        // format the name dropping 0 from the name array
        for (_, el) in id_array.into_iter().enumerate() {
            if *el == 0 {
                break;
            }
            index += 1
        }
        if let Ok(id) = std::str::from_utf8(&id_array[0..index]) {
            id.to_string()
        } else {
            String::from("UNKNOWN")
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

    #[cfg(unix)]
    pub fn bayer_pattern_to_str(n: &u32) -> &'static str {
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

    #[cfg(windows)]
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
        let num_of_devs = libasi::camera::get_num_of_connected_cameras();

        match num_of_devs {
            0 => warn!("No ZWO cameras found"),
            _ => info!("Found {} ZWO Cameras", num_of_devs),
        }
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

    /// This method looks for all control capabilities for the camera adn return them in
    /// a vector. Ideally this should be called only once when the camera is initialized.
    pub fn fetch_control_caps(num_of_caps: i32, cam_idx: i32) -> Vec<AsiProperty> {
        let mut caps: Vec<AsiProperty> = Vec::with_capacity(num_of_caps as usize);
        for i in 0..num_of_caps {
            let mut control_caps = AsiControlCaps::new();

            libasi::camera::get_control_caps(cam_idx, i, &mut control_caps);

            let cap = AsiProperty {
                name: crate::utils::asi_name_to_string_i8(&control_caps.Name).to_case(Case::Snake),
                _description: crate::utils::asi_name_to_string_i8(&control_caps.Description),
                _max_value: control_caps.MaxValue,
                _min_value: control_caps.MinValue,
                _default_value: control_caps.DefaultValue,
                _is_auto_supported: control_caps.IsAutoSupported != 0,
                is_writable: control_caps.IsWritable != 0,
                control_type: control_caps.ControlType as i32,
            };
            info!("Discovered capacity: {:?}", &cap.name);
            caps.push(cap);
        }

        caps
    }

    /// This method must be called AFTER the camera is initialized by the SDK
    pub fn get_num_of_controls(index: i32) -> i32 {
        let mut num_of_controls = 0;
        libasi::camera::get_num_of_controls(index, &mut num_of_controls);
        info!("Found: {} controls for camera {}", num_of_controls, index);
        num_of_controls
    }
}

// pub trait BaseAstroDevice {
//     /// Main and only entrypoint to create a new serial device.
//     ///
//     /// A device that doesn't work/cannot communicate with is not really useful
//     /// so this may return `None` if there is something wrong with the just
//     /// discovered device.
//     fn new(index: i32) -> Self
//     where
//         Self: Sized;

//     /// Use this method to fetch the real properties from the device,
//     /// this should not be called directly from clients ideally,
//     /// for that goal `get_properties` should be used.
//     fn fetch_props(&mut self);

//     /// Use this method to return the id of the device as a uuid.
//     fn get_id(&self) -> Uuid;

//     /// Use this method to return the name of the device (e.g. ZWO533MC).
//     fn get_name(&self) -> &String;

//     /// Use this method to return the actual cached state stored into `self.properties`.
//     //fn get_properties(&self) -> &Vec<Property>;

//     /// Method to be used when receving requests from clients to update properties.
//     ///
//     /// Ideally this should call internally `update_property_remote` which will be
//     /// responsible to trigger the action against the device to update the property
//     /// on the device itself, if the action is successful the last thing this method
//     /// does would be to update the property inside `self.properties`.
//     fn update_property(&mut self, prop_name: &str, val: &str) -> Result<(), DeviceActions>;

//     /// Method used internally by the driver itself to change values for properties that
//     /// be manipulated by the user (like the exposure ones)
//     fn update_internal_property(&mut self, prop_name: &str, val: &str)
//         -> Result<(), DeviceActions>;

//     /// Use this method to send a command to the device to change the requested property.
//     ///
//     /// Ideally this method will be a big `match` clause where the matching will execute
//     /// `self.send_command` to issue a serial command to the device.
//     fn update_property_remote(&mut self, prop_name: &str, val: &str) -> Result<(), DeviceActions>;

//     /// Properties are packed into a vector so to find them we need to
//     /// lookup the index, use this method to do so.
//     fn find_property_index(&self, prop_name: &str) -> Option<usize>;
// }
#[derive(Debug)]
pub struct AsiProperty {
    name: String,
    _description: String,
    _max_value: i64,
    _min_value: i64,
    _default_value: i64,
    _is_auto_supported: bool,
    is_writable: bool,
    control_type: i32,
}

#[derive(Debug, Serialize)]
pub struct CcdDevice {
    #[serde(skip)]
    pub id: Uuid,
    pub name: String,
    idx: i32,
    #[serde(skip)]
    caps: Vec<AsiProperty>,
    #[serde(flatten)]
    controls: std::collections::HashMap<String, Property<isize>>,
    #[serde(skip)]
    _ls_rand_id: [u8; 8],
    is_color: BoolProperty,
    camera_id: Property<u8>,
    max_height: Property<u16>,
    max_width: Property<u16>,
    bayer_pattern: Property<Cow<'static, str>>,
    bins: Property<Cow<'static, str>>,
    video_formats: Property<Cow<'static, str>>,
    pix_size: Property<f64>,
    has_shutter: BoolProperty,
    st4: BoolProperty,
    e_adu: Property<f32>,
    bit_depth: Property<u8>,
    lightspeed_id: Property<Cow<'static, str>>,
    // Properties to build logic around exposures
    exposing: BoolProperty,
    exposure_status: Property<Cow<'static, str>>,
    width: Property<i32>,
    height: Property<i32>,
    bin: Property<i32>,
    image_type: Property<i32>,
}

impl CcdDevice {
    pub fn new(index: i32) -> Self {
        // From the SDK documentation, in order:
        // 1) Get count of connected cameras (THIS IS DONE ALREADY as we already called look_for_devices
        // 2) get camera ID using ASIGetCameraProperty
        let mut info = AsiCameraInfo::new();
        libasi::camera::get_camera_info(&mut info, index);

        debug!(
            "Saying welcome to camera `{}`",
            utils::asi_name_to_string_i8(&info.Name)
        );

        // 3) Open camera using ASIOpenCamera
        libasi::camera::open_camera(index);

        // 4)Initialise the camera using ASIInitCamera
        libasi::camera::init_camera(index);

        // 5) Get count of control type with ASIGetControlCaps
        // Check how many capabilities this camera has, reallocate the vector
        // after the number is known
        let num_of_controls = get_num_of_controls(index);

        // Populate now the caps props as they won't change never during the camera's lifetime
        let caps = fetch_control_caps(num_of_controls, index);

        // Set the ROI
        //

        // Check if we have a random generated id for the camera, if not generate one,
        // store it on the camera itself and assign it to self.ls_rand_id
        let _ls_rand_id = utils::generics::get_camera_id(index);

        //for (i, byte) in ls_rand_id.as_bytes().iter().enumerate() {
        //    self.ls_rand_id[i] = *byte;
        //}

        let mut device = Self {
            id: Uuid::new_v4(),
            name: utils::asi_name_to_string_i8(&info.Name),
            idx: info.CameraID,
            caps,
            controls: HashMap::new(),
            _ls_rand_id: [0; 8],
            is_color: BoolProperty::new(info.IsColorCam == 1, Permission::ReadOnly),
            camera_id: Property::<u8>::new(info.CameraID as u8, Permission::ReadOnly, None),
            max_height: Property::<u16>::new(info.MaxHeight as u16, Permission::ReadOnly, None),
            max_width: Property::<u16>::new(info.MaxWidth as u16, Permission::ReadOnly, None),
            bayer_pattern: Property::<Cow<'static, str>>::new(
                Cow::Borrowed(&utils::bayer_pattern_to_str(&info.BayerPattern)),
                Permission::ReadOnly,
                None,
            ),
            bins: Property::<Cow<'static, str>>::new(
                Cow::Owned(utils::int_to_binning_str(&info.SupportedBins)),
                Permission::ReadOnly,
                None,
            ),
            video_formats: Property::<Cow<'static, str>>::new(
                Cow::Owned(utils::int_to_image_type_array(&info.SupportedVideoFormat)),
                Permission::ReadOnly,
                None,
            ),
            pix_size: Property::<f64>::new(info.PixelSize, Permission::ReadOnly, None),
            has_shutter: BoolProperty::new(info.MechanicalShutter == 1_u32, Permission::ReadOnly),
            st4: BoolProperty::new(info.ST4Port == 1_u32, Permission::ReadOnly),
            e_adu: Property::<f32>::new(info.ElecPerADU, Permission::ReadOnly, None),
            bit_depth: Property::<u8>::new(info.BitDepth as u8, Permission::ReadOnly, None),
            lightspeed_id: Property::<Cow<'static, str>>::new(
                Cow::Borrowed("lol"),
                Permission::ReadOnly,
                None,
            ),
            // Properties to build logic around exposures
            exposing: BoolProperty::new(false, Permission::ReadWrite),
            exposure_status: Property::<Cow<'static, str>>::new(
                Cow::Borrowed("IDLE"),
                Permission::ReadWrite,
                None,
            ),
            width: Property::new(0, Permission::ReadWrite, None),
            height: Property::new(0, Permission::ReadWrite, None),
            bin: Property::new(0, Permission::ReadWrite, None),
            image_type: Property::new(0, Permission::ReadWrite, None),
        };

        device.asi_caps_to_lightspeed_props();
        device.fetch_roi_format();
        device
    }

    pub fn fetch_props(&mut self) {
        let now = Instant::now();
        debug!("Fetching properties for device {}", self.name);

        for cap in &self.caps {
            let val = self.get_control_value(&cap);
            debug!("Cap {} value is  {}", &cap.name, &val);
            let v = self.controls.get_mut(&cap.name).unwrap();
            if v.value() != &val {
                v.set_val(val);
            }
        }

        let elapsed = now.elapsed();
        debug!("Elapsed: {:.2?}", elapsed);
    }

    /// Method to be used when receving requests from clients to update properties.
    ///
    /// Ideally this should call internally `update_property_remote` which will be
    /// responsible to trigger the action against the device to update the property
    /// on the device itself, if the action is successful the last thing this method
    /// does would be to update the property inside `self.properties`.
    fn update_property(&mut self, _prop_name: &str, _val: &str) -> Result<(), DeviceActions> {
        todo!();
    }

    fn index(&self) -> &i32 {
        &self.idx
    }

    fn asi_caps_to_lightspeed_props(&mut self) {
        for cap in &self.caps {
            debug!("CAP name: {}", &cap.name);
            let cap_value = self.get_control_value(cap);
            // here we create lightspeed properties from AsiCaps
            let prop = Property::<isize>::new(
                cap_value,
                if cap.is_writable {
                    Permission::ReadWrite
                } else {
                    Permission::ReadOnly
                },
                Some((
                    cap._min_value.try_into().unwrap(),
                    cap._max_value.try_into().unwrap(),
                )),
            );
            self.controls.insert(cap.name.to_owned(), prop);
        }
    }

    fn get_control_value(&self, cap: &AsiProperty) -> isize {
        debug!("Getting value for prop {}", cap.name);
        let mut is_auto_set = 0;
        let mut val: i64 = 0;

        libasi::camera::get_control_value(
            *self.index(),
            cap.control_type,
            &mut val,
            &mut is_auto_set,
        );
        debug!(
            "Value for {} is {} - Auto adjusted? {}",
            cap.name, val, cap.is_writable
        );
        val as isize
    }

    /// Close gently the connection to the camera using the SDK
    fn close(&self) {
        debug!("Closing camera {}", self.name);
        libasi::camera::close_camera(*self.index());
    }

    fn fetch_roi_format(&mut self) {
        info!("Reading ROI");
        let mut width = 0;
        let mut height = 0;
        let mut bin = 0;
        let mut img_type = 0;

        libasi::camera::get_roi_format(
            *self.index(),
            &mut width,
            &mut height,
            &mut bin,
            &mut img_type,
        );

        // Update now the struct values
        self.width.update(width).unwrap();
        self.height.update(height).unwrap();
        self.bin.update(bin).unwrap();
        self.image_type.update(img_type).unwrap();

        info!(
            "ROI format => width: {} | height: {} | bin: {} | img type: {}",
            self.width.value(),
            self.height.value(),
            self.bin.value(),
            self.image_type.value()
        );
    }
}
