use log::debug;

use asi_rs::{Manager, ZWOCCDManager};

fn main() {
    env_logger::init();

    let ccd_manager: ZWOCCDManager = Manager::new("asi-ccd");
    ccd_manager.init_provider();
    ccd_manager.get_devices_properties();

    for i in 0..ccd_manager.devices_discovered {
        debug!("Camera index: {}", i);
        let controls = ccd_manager.get_num_of_controls(i);
        ccd_manager.get_control_caps(i, controls);
        ccd_manager.get_roi_format(i);
    }

    ccd_manager.set_roi_format(0, 800, 600, 1, 0);
    ccd_manager.get_roi_format(0);
    ccd_manager.expose(0, 1.0);
    ccd_manager.close()
}
