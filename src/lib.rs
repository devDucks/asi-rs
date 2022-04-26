use dlopen::raw::Library;

struct Device {}

struct ZWOCCDManager {
    name: &'static str,
    shared_object: Library,
    devices: Vec<Device>,
}

#[repr(C)]
struct AsiCameraInfo {
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

trait Manager {
    fn new(&self, name: &'static str) -> Self;

    // Look on the system for devices and return an array
    // containing their indices
    fn look_for_devices(&self) -> i32;

    fn get_devices_properties(&self);

    fn init_provider(&self);

    fn pointer_to_vault(&self) -> Box<AsiCameraInfo>;
}

impl Manager for ZWOCCDManager {
    fn new(&self, name: &'static str) -> ZWOCCDManager {
        let lib = match Library::open("libASICamera2.so") {
            Ok(so) => so,
            Err(e) => panic!("{}", e),
        };
        let devices = usize::try_from(self.look_for_devices()).unwrap();
        ZWOCCDManager {
            name: name,
            shared_object: lib,
            devices: Vec::with_capacity(devices),
        }
    }

    fn pointer_to_vault(&self) -> Box<AsiCameraInfo> {
        return Box::new(AsiCameraInfo {
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
    }

    fn look_for_devices(&self) -> i32 {
        let look_for_devices: extern "C" fn() -> i32 =
            unsafe { self.shared_object.symbol("ASIGetNumOfConnectedCameras") }.unwrap();
        return look_for_devices();
    }

    fn get_devices_properties(&self) {
        let read_device_properties: extern "C" fn(*mut AsiCameraInfo, i32) -> i32 =
            unsafe { self.shared_object.symbol("ASIGetCameraProperty") }.unwrap();

        for index in 0..self.look_for_devices() {
            let mut vault = self.pointer_to_vault();
            match read_device_properties(&mut *vault, index) {
                0 => println!("Properties retrieved correctly"),
                e => panic!("Error happened: {}", e),
            }
        }
    }

    fn init_provider(&self) {}
}
