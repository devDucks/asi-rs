#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod camera {
    include!(concat!(env!("OUT_DIR"), "/cam_bindings.rs"));

    impl _ASI_ID {
        pub fn new() -> Self {
            Self { id: [0; 8] }
        }
    }

    impl _ASI_CAMERA_INFO {
        pub fn new() -> Self {
            Self {
                Name: [0; 64],
                CameraID: 0,
                MaxHeight: 0,
                MaxWidth: 0,
                IsColorCam: 1,
                BayerPattern: 1,
                SupportedBins: [0; 16],
                SupportedVideoFormat: [0; 8],
                PixelSize: 0.0,
                MechanicalShutter: 0,
                ST4Port: 0,
                IsCoolerCam: 0,
                IsUSB3Host: 0,
                IsUSB3Camera: 0,
                ElecPerADU: 0.0,
                BitDepth: 0,
                IsTriggerCam: 0,
                Unused: [0; 16],
            }
        }
    }

    impl _ASI_CONTROL_CAPS {
        pub fn new() -> Self {
            Self {
                Name: [0; 64],
                Description: [0; 128],
                MaxValue: 0,
                MinValue: 0,
                DefaultValue: 0,
                IsAutoSupported: 0,
                IsWritable: 0,
                ControlType: 0,
                Unused: [0; 32],
            }
        }
    }
}

pub mod efw {
    include!(concat!(env!("OUT_DIR"), "/efw_bindings.rs"));

    impl _EFW_ID {
        pub fn new() -> Self {
	    Self { id: [0; 8] }
        }
    }

    impl _EFW_INFO {
	pub fn new() -> Self {
	    Self {
		ID: 0,
		Name: [0; 64],
		slotNum: 0,
	    }
	}
    }
}
