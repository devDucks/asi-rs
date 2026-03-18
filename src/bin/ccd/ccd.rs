use libasi::camera::{AsiCameraInfo, AsiControlCaps, CameraHardware, ROIFormat};

use astrotools::properties::{Permission, Prop, Property, RangeProperty};
use log::{debug, error, info};
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use uuid::Uuid;

type Camera = Arc<RwLock<AsiCamera>>;

pub mod utils {
    use crate::ccd::AsiProperty;
    use convert_case::{Case, Casing};
    use libasi::camera::{AsiControlCaps, CameraHardware};
    use log::{error, info, warn};

    pub mod generics {
        use libasi::camera::{AsiID, CameraHardware};
        use log::{debug, info};
        use rand::distributions::Alphanumeric;
        use rand::{thread_rng, Rng};

        pub fn get_camera_id(camera_index: i32, hw: &dyn CameraHardware) -> String {
            let id = hw
                .get_cam_id(camera_index)
                .unwrap_or_else(|e| {
                    error!("get_cam_id failed: {:?}", e);
                    AsiID::new()
                });

            if id.id == [0, 0, 0, 0, 0, 0, 0, 0] {
                debug!("Setting a random uid");
                set_camera_id(camera_index, None, hw);
            }
            let id_str = asi_rs::utils::asi_id_to_string(&id.id);
            info!("ASI ID for camera with index {}: {:?}", camera_index, &id);
            id_str
        }

        pub fn set_camera_id(
            camera_index: i32,
            cam_id: Option<[u8; 8]>,
            hw: &dyn CameraHardware,
        ) {
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
                asi_rs::utils::asi_id_to_string(&id.id)
            );
            hw.set_cam_id(camera_index, id)
                .unwrap_or_else(|e| error!("set_cam_id failed: {:?}", e));
        }
    }

    pub mod capturing {
        use crate::ccd::{camera_image_buffer_size, Camera};
        use astrotools::properties::Prop;
        use base64::prelude::BASE64_STANDARD;
        use base64::Engine;
        use libasi::camera::CameraHardware;
        use log::{debug, error, info};
        use rumqttc::Event::Incoming;
        use rumqttc::{Client, MqttOptions};
        use std::sync::Arc;
        use std::time::Duration;
        use std::time::SystemTime;

        pub fn expose(length: f32, img_type: i32, device: Camera) {
            let (width, height, idx, hw) = {
                let r = device.read().unwrap();
                (
                    *r.width.value(),
                    *r.height.value(),
                    r.idx,
                    Arc::clone(&r.hw),
                )
            };

            let buffer_size = match camera_image_buffer_size(width, height, img_type) {
                Some(s) => s,
                None => {
                    error!("Unsupported image type {}, aborting exposure", img_type);
                    return;
                }
            };

            let secs_to_micros: i64 = (length * 1_000_000_f32) as i64;
            let mut image_buffer = vec![0u8; buffer_size as usize];

            debug!("Update prop exposing {}", secs_to_micros);

            hw.set_control_value(
                idx,
                libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32,
                secs_to_micros,
                libasi::camera::ASI_BOOL_ASI_FALSE as i32,
            )
            .unwrap_or_else(|e| error!("set_control_value failed: {:?}", e));

            hw.start_exposure(idx)
                .unwrap_or_else(|e| error!("start_exposure failed: {:?}", e));

            let mut status = hw.exposure_status(idx).unwrap_or_else(|e| {
                error!("exposure_status failed: {:?}", e);
                0
            });

            let start = SystemTime::now();
            {
                let mut d = device.write().unwrap();
                d.exposure_status
                    .update_int(std::borrow::Cow::Borrowed("EXPOSING"));
            }

            debug!("Started exposure");

            while status == 1 {
                status = hw.exposure_status(idx).unwrap_or_else(|e| {
                    error!("exposure_status failed: {:?}", e);
                    0
                });
                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            info!("Elapsed: {}", start.elapsed().unwrap().as_micros());

            match status {
                libasi::camera::ASI_EXPOSURE_STATUS_ASI_EXP_SUCCESS => {
                    info!("Exposure successful");
                    {
                        let mut d = device.write().unwrap();
                        d.exposure_status
                            .update_int(std::borrow::Cow::Borrowed("SUCCESS"));
                    }

                    info!("downloading");
                    hw.download_exposure(idx, &mut image_buffer)
                        .unwrap_or_else(|e| error!("download_exposure failed: {:?}", e));

                    let mut mqttoptions = MqttOptions::new("asi_exposure", "127.0.0.1", 1883);
                    mqttoptions.set_keep_alive(Duration::from_secs(5));
                    let (mut client, mut connection) = Client::new(mqttoptions, 10);

                    client
                        .publish(
                            format!(
                                "{}",
                                format_args!(
                                    "devices/{}/exposure",
                                    &device.read().unwrap().id.to_string()
                                )
                            ),
                            rumqttc::QoS::AtLeastOnce,
                            false,
                            BASE64_STANDARD.encode(&image_buffer),
                        )
                        .unwrap();

                    for notification in connection.iter() {
                        match notification {
                            Ok(Incoming(inc)) => match inc {
                                rumqttc::Packet::PubAck(_m) => break,
                                _ => continue,
                            },
                            _ => continue,
                        }
                    }
                }
                libasi::camera::ASI_EXPOSURE_STATUS_ASI_EXP_FAILED => {
                    error!("Exposure failed")
                }
                n => error!("An error happened: {}", n),
            }
        }
    }

    pub fn check_error_code(code: i32) {
        match code {
            0 => (),
            1 => error!("ASI_ERROR_INVALID_INDEX"),
            2 => error!("ASI_ERROR_INVALID_ID"),
            3 => error!("ASI_ERROR_INVALID_CONTROL_TYPE"),
            4 => error!("ASI_ERROR_CAMERA_CLOSED"),
            5 => error!("ASI_ERROR_CAMERA_REMOVED"),
            6 => error!("ASI_ERROR_INVALID_PATH"),
            7 => error!("ASI_ERROR_INVALID_FILEFORMAT"),
            8 => error!("ASI_ERROR_INVALID_SIZE"),
            9 => error!("ASI_ERROR_INVALID_IMGTYPE"),
            10 => error!("ASI_ERROR_OUTOF_BOUNDARY"),
            11 => error!("ASI_ERROR_TIMEOUT"),
            12 => error!("ASI_ERROR_INVALID_SEQUENCE"),
            13 => error!("ASI_ERROR_BUFFER_TOO_SMALL"),
            14 => error!("ASI_ERROR_VIDEO_MODE_ACTIVE"),
            15 => error!("ASI_ERROR_EXPOSURE_IN_PROGRESS"),
            16 => error!("ASI_ERROR_GENERAL_ERROR"),
            17 => error!("ASI_ERROR_INVALID_MODE"),
            18 => error!("ASI_ERROR_END"),
            e => error!("unknown error {}", e),
        }
    }

    pub fn bayer_pattern_to_str(n: &u32) -> &'static str {
        match n {
            0 => "RG",
            1 => "BG",
            2 => "GR",
            3 => "GB",
            _ => {
                error!("Bayer pattern not recognized");
                "UNKNOWN"
            }
        }
    }

    pub fn look_for_devices(hw: &dyn CameraHardware) -> i32 {
        let num_of_devs = hw.get_num_of_connected_cameras();
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

    /// This method looks for all control capabilities for the camera and returns them in
    /// a vector. Ideally this should be called only once when the camera is initialized.
    pub fn fetch_control_caps(
        num_of_caps: i32,
        cam_idx: i32,
        hw: &dyn CameraHardware,
    ) -> Vec<AsiProperty> {
        let mut caps: Vec<AsiProperty> = Vec::with_capacity(num_of_caps as usize);
        for i in 0..num_of_caps {
            let mut control_caps = AsiControlCaps::new();
            hw.get_control_caps(cam_idx, i, &mut control_caps)
                .unwrap_or_else(|e| error!("get_control_caps failed: {:?}", e));

            let cap = AsiProperty {
                name: asi_rs::utils::asi_name_to_string(&control_caps.Name)
                    .to_case(Case::Snake),
                _description: asi_rs::utils::asi_name_to_string(&control_caps.Description),
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

    /// This method must be called AFTER the camera is initialized by the SDK.
    pub fn get_num_of_controls(index: i32, hw: &dyn CameraHardware) -> i32 {
        let num = hw
            .get_num_of_controls(index)
            .unwrap_or_else(|e| {
                error!("get_num_of_controls failed: {:?}", e);
                0
            });
        info!("Found: {} controls for camera {}", num, index);
        num
    }
}

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
pub struct AsiCamera {
    #[serde(skip)]
    pub id: Uuid,
    pub name: String,
    idx: i32,
    #[serde(skip)]
    caps: Vec<AsiProperty>,
    #[serde(flatten)]
    controls: std::collections::HashMap<String, RangeProperty<isize>>,
    #[serde(skip)]
    _ls_rand_id: [u8; 8],
    #[serde(skip)]
    pub hw: Arc<dyn CameraHardware>,
    is_color: Property<bool>,
    camera_id: Property<u8>,
    max_height: Property<u16>,
    max_width: Property<u16>,
    bayer_pattern: Property<Cow<'static, str>>,
    bins: Property<Cow<'static, str>>,
    video_formats: Property<Cow<'static, str>>,
    pix_size: Property<f64>,
    has_shutter: Property<bool>,
    st4: Property<bool>,
    e_adu: Property<f32>,
    bit_depth: Property<u8>,
    lightspeed_id: Property<Cow<'static, str>>,
    exposing: Property<bool>,
    pub exposure_status: Property<Cow<'static, str>>,
    pub width: Property<i32>,
    pub height: Property<i32>,
    bin: Property<i32>,
    image_type: Property<i32>,
}

impl AsiCamera {
    pub fn new(index: i32, hw: Arc<dyn CameraHardware>) -> Self {
        let mut info = AsiCameraInfo::new();
        hw.get_camera_info(&mut info, index)
            .unwrap_or_else(|e| error!("get_camera_info failed: {:?}", e));

        debug!(
            "Saying welcome to camera `{}`",
            asi_rs::utils::asi_name_to_string(&info.Name)
        );

        hw.open_camera(index)
            .unwrap_or_else(|e| error!("open_camera failed: {:?}", e));
        hw.init_camera(index)
            .unwrap_or_else(|e| error!("init_camera failed: {:?}", e));

        let num_of_controls = utils::get_num_of_controls(index, hw.as_ref());
        let caps = utils::fetch_control_caps(num_of_controls, index, hw.as_ref());

        let _ls_rand_id = utils::generics::get_camera_id(index, hw.as_ref());

        let mut device = Self {
            id: Uuid::new_v4(),
            name: asi_rs::utils::asi_name_to_string(&info.Name),
            idx: info.CameraID,
            caps,
            controls: HashMap::new(),
            _ls_rand_id: [0; 8],
            hw,
            is_color: Property::new(info.IsColorCam == 1, Permission::ReadOnly),
            camera_id: Property::<u8>::new(info.CameraID as u8, Permission::ReadOnly),
            max_height: Property::<u16>::new(info.MaxHeight as u16, Permission::ReadOnly),
            max_width: Property::<u16>::new(info.MaxWidth as u16, Permission::ReadOnly),
            bayer_pattern: Property::<Cow<'static, str>>::new(
                Cow::Borrowed(utils::bayer_pattern_to_str(&info.BayerPattern)),
                Permission::ReadOnly,
            ),
            bins: Property::<Cow<'static, str>>::new(
                Cow::Owned(utils::int_to_binning_str(&info.SupportedBins)),
                Permission::ReadOnly,
            ),
            video_formats: Property::<Cow<'static, str>>::new(
                Cow::Owned(utils::int_to_image_type_array(&info.SupportedVideoFormat)),
                Permission::ReadOnly,
            ),
            pix_size: Property::<f64>::new(info.PixelSize, Permission::ReadOnly),
            has_shutter: Property::new(info.MechanicalShutter == 1_u32, Permission::ReadOnly),
            st4: Property::new(info.ST4Port == 1_u32, Permission::ReadOnly),
            e_adu: Property::<f32>::new(info.ElecPerADU, Permission::ReadOnly),
            bit_depth: Property::<u8>::new(info.BitDepth as u8, Permission::ReadOnly),
            lightspeed_id: Property::<Cow<'static, str>>::new(
                Cow::Borrowed("lol"),
                Permission::ReadOnly,
            ),
            exposing: Property::new(false, Permission::ReadOnly),
            exposure_status: Property::<Cow<'static, str>>::new(
                Cow::Borrowed("IDLE"),
                Permission::ReadOnly,
            ),
            width: Property::new(0, Permission::ReadWrite),
            height: Property::new(0, Permission::ReadWrite),
            bin: Property::new(0, Permission::ReadWrite),
            image_type: Property::new(0, Permission::ReadWrite),
        };

        device.asi_caps_to_lightspeed_props();
        device.fetch_roi_format();
        device
    }

    pub fn fetch_props(&mut self) {
        let now = Instant::now();
        debug!("Fetching properties for device {}", self.name);

        for cap in &self.caps {
            let val = self.get_control_value(cap);
            debug!("Cap {} value is  {}", &cap.name, &val);
            let v = self.controls.get_mut(&cap.name).unwrap();
            if v.value() != &val {
                v.update_int(val);
            }
        }

        let elapsed = now.elapsed();
        debug!("Elapsed: {:.2?}", elapsed);
    }

    pub fn update_property(&mut self, prop_name: &str, val: i32) {
        info!("UPDATE: prop name {}", &prop_name);
        info!("UPDATE: val {}", &val);
        match prop_name {
            "img_type" => {
                self.set_roi_format(None, None, None, Some(val));
                self.fetch_roi_format();
            }
            _ => error!("Unknown property: {}", prop_name),
        }
    }

    fn index(&self) -> &i32 {
        &self.idx
    }

    fn asi_caps_to_lightspeed_props(&mut self) {
        for cap in &self.caps {
            debug!("CAP name: {}", &cap.name);
            let cap_value = self.get_control_value(cap);
            let prop = RangeProperty::<isize>::new(
                cap_value,
                if cap.is_writable {
                    Permission::ReadWrite
                } else {
                    Permission::ReadOnly
                },
                cap._min_value.try_into().unwrap(),
                cap._max_value.try_into().unwrap(),
            );
            self.controls.insert(cap.name.to_owned(), prop);
        }
    }

    fn get_control_value(&self, cap: &AsiProperty) -> isize {
        debug!("Getting value for prop {}", cap.name);
        let val = self
            .hw
            .get_control_value(*self.index(), cap.control_type)
            .unwrap_or_else(|e| {
                error!("get_control_value for {} failed: {:?}", cap.name, e);
                0
            });
        debug!(
            "Value for {} is {} - Writable? {}",
            cap.name, val, cap.is_writable
        );
        val as isize
    }

    pub fn close(&self) {
        debug!("Closing camera {}", self.name);
        self.hw
            .close_camera(*self.index())
            .unwrap_or_else(|e| error!("close_camera failed: {:?}", e));
    }

    fn fetch_roi_format(&mut self) {
        info!("Reading ROI");
        let roi = self
            .hw
            .get_roi_format(*self.index())
            .unwrap_or_else(|e| {
                error!("get_roi_format failed: {:?}", e);
                ROIFormat {
                    width: 0,
                    height: 0,
                    bin: 0,
                    img_type: 0,
                }
            });

        self.width.update(roi.width).unwrap();
        self.height.update(roi.height).unwrap();
        self.bin.update(roi.bin).unwrap();
        self.image_type.update(roi.img_type).unwrap();

        info!(
            "ROI format => width: {} | height: {} | bin: {} | img type: {}",
            self.width.value(),
            self.height.value(),
            self.bin.value(),
            self.image_type.value()
        );
    }

    fn set_roi_format(
        &self,
        width: Option<i32>,
        height: Option<i32>,
        bin: Option<i32>,
        img_type: Option<i32>,
    ) {
        info!("Setting ROI");
        let roi = ROIFormat {
            width: width.unwrap_or_else(|| *self.width.value()),
            height: height.unwrap_or_else(|| *self.height.value()),
            bin: bin.unwrap_or_else(|| *self.bin.value()),
            img_type: img_type.unwrap_or_else(|| *self.image_type.value()),
        };
        self.hw
            .set_roi_format(*self.index(), roi)
            .unwrap_or_else(|e| error!("set_roi_format failed: {:?}", e));
    }
}

/// Returns the byte buffer size required for one frame, or `None` for unknown formats.
///
/// Image type values (from the ASI SDK): RAW8=0, RGB24=1, RAW16=2, Y8=3.
pub fn camera_image_buffer_size(width: i32, height: i32, img_type: i32) -> Option<i32> {
    match img_type {
        1 => Some(width * height * 3), // RGB24
        2 => Some(width * height * 2), // RAW16
        0 | 3 => Some(width * height), // RAW8 or Y8
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::utils::*;
    use super::*;
    use libasi::camera::{AsiControlCaps, AsiError, AsiID, CameraHardware, ROIFormat};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // -----------------------------------------------------------------------
    // Mock hardware implementation
    // -----------------------------------------------------------------------

    struct MockCamera {
        roi: Mutex<ROIFormat>,
        control_values: Mutex<HashMap<i32, i64>>,
    }

    impl MockCamera {
        fn new() -> Self {
            MockCamera {
                roi: Mutex::new(ROIFormat {
                    width: 1920,
                    height: 1080,
                    bin: 1,
                    img_type: 0, // RAW8
                }),
                control_values: Mutex::new(HashMap::new()),
            }
        }
    }

    impl CameraHardware for MockCamera {
        fn get_num_of_connected_cameras(&self) -> i32 {
            1
        }
        fn get_camera_info(
            &self,
            _info: &mut libasi::camera::AsiCameraInfo,
            _index: i32,
        ) -> Result<(), AsiError> {
            Ok(())
        }
        fn open_camera(&self, _index: i32) -> Result<(), AsiError> {
            Ok(())
        }
        fn init_camera(&self, _index: i32) -> Result<(), AsiError> {
            Ok(())
        }
        fn close_camera(&self, _index: i32) -> Result<(), AsiError> {
            Ok(())
        }
        fn get_num_of_controls(&self, _index: i32) -> Result<i32, AsiError> {
            Ok(0)
        }
        fn get_control_caps(
            &self,
            _camera_id: i32,
            _cap_index: i32,
            _caps: &mut AsiControlCaps,
        ) -> Result<(), AsiError> {
            Ok(())
        }
        fn get_control_value(
            &self,
            _camera_index: i32,
            control_type: i32,
        ) -> Result<i64, AsiError> {
            Ok(*self
                .control_values
                .lock()
                .unwrap()
                .get(&control_type)
                .unwrap_or(&0))
        }
        fn set_control_value(
            &self,
            _camera_index: i32,
            control_type: i32,
            value: i64,
            _is_auto_set: i32,
        ) -> Result<(), AsiError> {
            self.control_values
                .lock()
                .unwrap()
                .insert(control_type, value);
            Ok(())
        }
        fn get_roi_format(&self, _camera_id: i32) -> Result<ROIFormat, AsiError> {
            Ok(*self.roi.lock().unwrap())
        }
        fn set_roi_format(
            &self,
            _camera_id: i32,
            roi: ROIFormat,
        ) -> Result<(), AsiError> {
            *self.roi.lock().unwrap() = roi;
            Ok(())
        }
        fn get_cam_id(&self, _camera_id: i32) -> Result<AsiID, AsiError> {
            Ok(AsiID::new())
        }
        fn set_cam_id(&self, _camera_id: i32, _asi_id: AsiID) -> Result<(), AsiError> {
            Ok(())
        }
        fn start_exposure(&self, _camera_id: i32) -> Result<(), AsiError> {
            Ok(())
        }
        fn stop_exposure(&self, _camera_id: i32) -> Result<(), AsiError> {
            Ok(())
        }
        fn exposure_status(&self, _camera_id: i32) -> Result<u32, AsiError> {
            Ok(2) // ASI_EXPOSURE_STATUS_ASI_EXP_SUCCESS
        }
        fn download_exposure(
            &self,
            _camera_id: i32,
            _buffer: &mut [u8],
        ) -> Result<(), AsiError> {
            Ok(())
        }
        fn get_start_position(&self, _cam_idx: i32) -> Result<(i32, i32), AsiError> {
            Ok((0, 0))
        }
        fn get_camera_mode(&self, _cam_idx: i32) -> Result<i32, AsiError> {
            Ok(0)
        }
    }

    // -----------------------------------------------------------------------
    // camera_image_buffer_size tests (step 1: extract and test)
    // -----------------------------------------------------------------------

    // img_type integer values from the SDK: RAW8=0, RGB24=1, RAW16=2, Y8=3
    #[test]
    fn test_buffer_size_raw8() {
        assert_eq!(camera_image_buffer_size(1920, 1080, 0), Some(1920 * 1080));
    }

    #[test]
    fn test_buffer_size_rgb24() {
        assert_eq!(
            camera_image_buffer_size(1920, 1080, 1),
            Some(1920 * 1080 * 3)
        );
    }

    #[test]
    fn test_buffer_size_raw16() {
        assert_eq!(
            camera_image_buffer_size(1920, 1080, 2),
            Some(1920 * 1080 * 2)
        );
    }

    #[test]
    fn test_buffer_size_y8() {
        assert_eq!(camera_image_buffer_size(640, 480, 3), Some(640 * 480));
    }

    #[test]
    fn test_buffer_size_unknown_returns_none() {
        // Previously this was a todo!() which panicked at runtime.
        // Now it returns None safely.
        assert_eq!(camera_image_buffer_size(1920, 1080, 99), None);
        assert_eq!(camera_image_buffer_size(1920, 1080, -2), None);
    }

    // -----------------------------------------------------------------------
    // set_roi_format option defaulting tests (via mock)
    // -----------------------------------------------------------------------

    fn make_camera() -> AsiCamera {
        let hw = Arc::new(MockCamera::new());
        AsiCamera::new(0, hw)
    }

    #[test]
    fn test_fetch_roi_format_populates_fields() {
        let cam = make_camera();
        assert_eq!(*cam.width.value(), 1920);
        assert_eq!(*cam.height.value(), 1080);
        assert_eq!(*cam.bin.value(), 1);
    }

    #[test]
    fn test_set_roi_format_some_overrides_stored_value() {
        let cam = make_camera();
        // Provide all four override values
        cam.set_roi_format(Some(640), Some(480), Some(2), Some(0));
        let roi = cam.hw.get_roi_format(cam.idx).unwrap();
        assert_eq!(roi.width, 640);
        assert_eq!(roi.height, 480);
        assert_eq!(roi.bin, 2);
        assert_eq!(roi.img_type, 0);
    }

    #[test]
    fn test_set_roi_format_none_keeps_stored_values() {
        let cam = make_camera();
        // No overrides — should write back the current stored values
        cam.set_roi_format(None, None, None, None);
        let roi = cam.hw.get_roi_format(cam.idx).unwrap();
        assert_eq!(roi.width, *cam.width.value());
        assert_eq!(roi.height, *cam.height.value());
    }

    #[test]
    fn test_set_roi_format_partial_override() {
        let cam = make_camera();
        // Override only width; height/bin/img_type stay at mock values
        cam.set_roi_format(Some(320), None, None, None);
        let roi = cam.hw.get_roi_format(cam.idx).unwrap();
        assert_eq!(roi.width, 320);
        assert_eq!(roi.height, *cam.height.value());
    }

    // -----------------------------------------------------------------------
    // Pure utility function tests (int_to_binning_str, int_to_image_type, etc.)
    // -----------------------------------------------------------------------

    #[test]
    fn test_int_to_binning_str_single_value() {
        assert_eq!(int_to_binning_str(&[1, 0, 0, 0]), "1x1");
    }

    #[test]
    fn test_int_to_binning_str_multiple_values() {
        assert_eq!(int_to_binning_str(&[1, 2, 3, 0]), "1x1,2x2,3x3");
    }

    #[test]
    fn test_int_to_binning_str_stops_at_zero() {
        assert_eq!(int_to_binning_str(&[1, 2, 0, 4]), "1x1,2x2");
    }

    #[test]
    fn test_int_to_binning_str_all_values() {
        assert_eq!(int_to_binning_str(&[1, 2, 3, 4]), "1x1,2x2,3x3,4x4");
    }

    #[test]
    fn test_int_to_image_type_raw8() {
        assert_eq!(int_to_image_type(0), "RAW8");
    }

    #[test]
    fn test_int_to_image_type_rgb24() {
        assert_eq!(int_to_image_type(1), "RGB24");
    }

    #[test]
    fn test_int_to_image_type_raw16() {
        assert_eq!(int_to_image_type(2), "RAW16");
    }

    #[test]
    fn test_int_to_image_type_y8() {
        assert_eq!(int_to_image_type(3), "Y8");
    }

    #[test]
    fn test_int_to_image_type_end() {
        assert_eq!(int_to_image_type(-1), "END");
    }

    #[test]
    fn test_int_to_image_type_unknown() {
        assert_eq!(int_to_image_type(99), "UNKNOWN");
    }

    #[test]
    fn test_int_to_image_type_array_stops_at_minus_one() {
        assert_eq!(int_to_image_type_array(&[0, 1, -1, 2]), "RAW8,RGB24");
    }

    #[test]
    fn test_int_to_image_type_array_single() {
        assert_eq!(int_to_image_type_array(&[2, -1]), "RAW16");
    }

    #[test]
    fn test_int_to_image_type_array_empty_sentinel_first() {
        assert_eq!(int_to_image_type_array(&[-1, 0, 1]), "");
    }

    #[test]
    fn test_int_to_image_type_array_all_types() {
        assert_eq!(
            int_to_image_type_array(&[0, 1, 2, 3, -1]),
            "RAW8,RGB24,RAW16,Y8"
        );
    }

    #[test]
    fn test_bayer_pattern_rg() {
        assert_eq!(bayer_pattern_to_str(&0u32), "RG");
    }

    #[test]
    fn test_bayer_pattern_bg() {
        assert_eq!(bayer_pattern_to_str(&1u32), "BG");
    }

    #[test]
    fn test_bayer_pattern_gr() {
        assert_eq!(bayer_pattern_to_str(&2u32), "GR");
    }

    #[test]
    fn test_bayer_pattern_gb() {
        assert_eq!(bayer_pattern_to_str(&3u32), "GB");
    }

    #[test]
    fn test_bayer_pattern_unknown() {
        assert_eq!(bayer_pattern_to_str(&99u32), "UNKNOWN");
    }
}
