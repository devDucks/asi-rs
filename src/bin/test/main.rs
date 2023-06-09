use std::time::Instant;

fn get_roi(idx: i32) {
    let mut width = 20;
    let mut height = 20;
    let mut bin = 20;
    let mut img_type = 20;
    libasi::camera::get_roi_format(idx, &mut width, &mut height, &mut bin, &mut img_type);
    println!(
        "Width: {}\nHeight: {}\nBin: {}\nType: {}",
        width, height, bin, img_type
    )
}

fn expose(idx: i32) -> u32 {
    let mut e_val = 0;
    libasi::camera::get_control_value(
        idx,
        libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32,
        &mut e_val,
        &mut 0,
    );
    println!("Exp time: {}", e_val);
    println!("Exposing");
    libasi::camera::start_exposure(idx);

    let mut status = 0;
    libasi::camera::exposure_status(idx, &mut status);

    while status == 1 {
        libasi::camera::exposure_status(idx, &mut status);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    println!("Exposure status: {}", &status);
    status
}

fn main() {
    let num_of_devs = libasi::camera::get_num_of_connected_cameras();
    println!("Found {} camera(s)", &num_of_devs);

    for idx in 0..num_of_devs {
        println!("Probing camera {}", &idx);
        libasi::camera::open_camera(idx);
        libasi::camera::init_camera(idx);

        let mut num_of_controls = 0;
        libasi::camera::get_num_of_controls(idx, &mut num_of_controls);
        println!("Found: {} controls for camera {}", num_of_controls, idx);

        let mut caps = Vec::with_capacity(num_of_controls as usize);

        for c_id in 0..num_of_controls {
            let mut control_caps = libasi::camera::AsiControlCaps::new();
            libasi::camera::get_control_caps(idx, c_id, &mut control_caps);
            caps.push(control_caps);
        }

        let mut sum = std::time::Duration::new(0, 0);

        for _ in 0..50 {
            let now = Instant::now();
            for cap in &caps {
                let mut is_auto_set = 0;
                let mut val: i64 = 0;

                libasi::camera::get_control_value(
                    idx,
                    cap.ControlType as i32,
                    &mut val,
                    &mut is_auto_set,
                );
            }
            let elapsed = now.elapsed();
            //println!("Reading all props took: {:.2?}", elapsed);
            sum += elapsed;
        }

        println!("Run average: {:.2?}", sum / 50);

        get_roi(idx);
        //libasi::camera::set_roi_format(idx, 64, 64, 1, 1);
        //get_roi(idx);

        let mut start_x = 50;
        let mut start_y = 50;

        libasi::camera::get_start_position(idx, &mut start_x, &mut start_y);
        println!("Start X: {}, Start Y: {}", start_x, start_y);

        let mut e_val = 0;
        libasi::camera::get_control_value(
            idx,
            libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32,
            &mut e_val,
            &mut 0,
        );
        println!("Exp before: {}", e_val);

        let length: ::std::os::raw::c_long = 10_000_000;

        let mut cmode = 100;

        libasi::camera::get_camera_mode(idx, &mut cmode);
        println!("Camera mode: {}", cmode);

        libasi::camera::set_control_value(
            idx,
            libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32,
            length,
            libasi::camera::ASI_BOOL_ASI_FALSE as i32,
        );
        let mut e_val = 0;
        libasi::camera::get_control_value(
            idx,
            libasi::camera::ASI_CONTROL_TYPE_ASI_EXPOSURE as i32,
            &mut e_val,
            &mut 0,
        );
        println!("Exp after: {}", e_val);

        let mut counter = 0;

        while expose(idx) == 3_u32 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            expose(idx);
            counter += 1;

            if counter > 5 {
                break;
            }
        }

        libasi::camera::close_camera(idx);
    }
}
