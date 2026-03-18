use libasi::camera::{CameraHardware, RealCamera};
use std::time::Instant;

fn get_roi(idx: i32, hw: &dyn CameraHardware) {
    match hw.get_roi_format(idx) {
        Ok(roi) => println!(
            "Width: {}\nHeight: {}\nBin: {}\nType: {}",
            roi.width, roi.height, roi.bin, roi.img_type
        ),
        Err(e) => eprintln!("get_roi_format failed: {:?}", e),
    }
}

fn expose(idx: i32, hw: &dyn CameraHardware) -> u32 {
    match hw.get_control_value(idx, libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32) {
        Ok(v) => println!("Exp time: {}", v),
        Err(e) => eprintln!("get_control_value failed: {:?}", e),
    }

    println!("Exposing");
    hw.start_exposure(idx)
        .unwrap_or_else(|e| eprintln!("start_exposure failed: {:?}", e));

    let mut status = hw.exposure_status(idx).unwrap_or(0);

    while status == 1 {
        status = hw.exposure_status(idx).unwrap_or(0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    println!("Exposure status: {}", &status);
    status
}

fn main() {
    let hw = RealCamera;
    let num_of_devs = hw.get_num_of_connected_cameras();
    println!("Found {} camera(s)", &num_of_devs);

    for idx in 0..num_of_devs {
        println!("Probing camera {}", &idx);
        hw.open_camera(idx)
            .unwrap_or_else(|e| eprintln!("open_camera failed: {:?}", e));
        hw.init_camera(idx)
            .unwrap_or_else(|e| eprintln!("init_camera failed: {:?}", e));

        let num_of_controls = hw.get_num_of_controls(idx).unwrap_or(0);
        println!("Found: {} controls for camera {}", num_of_controls, idx);

        let mut caps = Vec::with_capacity(num_of_controls as usize);
        for c_id in 0..num_of_controls {
            let mut control_caps = libasi::camera::AsiControlCaps::new();
            hw.get_control_caps(idx, c_id, &mut control_caps)
                .unwrap_or_else(|e| eprintln!("get_control_caps failed: {:?}", e));
            caps.push(control_caps);
        }

        let mut sum = std::time::Duration::new(0, 0);
        for _ in 0..50 {
            let now = Instant::now();
            for cap in &caps {
                hw.get_control_value(idx, cap.ControlType as i32).ok();
            }
            sum += now.elapsed();
        }
        println!("Run average: {:.2?}", sum / 50);

        get_roi(idx, &hw);

        match hw.get_start_position(idx) {
            Ok((x, y)) => println!("Start X: {}, Start Y: {}", x, y),
            Err(e) => eprintln!("get_start_position failed: {:?}", e),
        }

        match hw.get_control_value(idx, libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32) {
            Ok(v) => println!("Exp before: {}", v),
            Err(e) => eprintln!("get_control_value failed: {:?}", e),
        }

        match hw.get_camera_mode(idx) {
            Ok(m) => println!("Camera mode: {}", m),
            Err(e) => eprintln!("get_camera_mode failed: {:?}", e),
        }

        let length: i64 = 10_000_000;
        hw.set_control_value(
            idx,
            libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32,
            length,
            libasi::camera::ASI_BOOL_ASI_FALSE as i32,
        )
        .unwrap_or_else(|e| eprintln!("set_control_value failed: {:?}", e));

        match hw.get_control_value(idx, libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32) {
            Ok(v) => println!("Exp after: {}", v),
            Err(e) => eprintln!("get_control_value failed: {:?}", e),
        }

        let mut counter = 0;
        while expose(idx, &hw) == 3_u32 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            expose(idx, &hw);
            counter += 1;
            if counter > 5 {
                break;
            }
        }

        hw.close_camera(idx)
            .unwrap_or_else(|e| eprintln!("close_camera failed: {:?}", e));
    }
}
