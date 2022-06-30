use log::{info, warn};

pub fn look_for_devices() -> i32 {
    let num_of_devs = libasi::efw::get_num_of_connected_devices();

    match num_of_devs {
        0 => warn!("No ZWO EFW found"),
        _ => info!("Found {} ZWO EFW(s)", num_of_devs),
    }
    num_of_devs
}
