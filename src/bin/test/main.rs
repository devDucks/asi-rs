use std::time::Instant;

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

        for i in 0..50 {
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
            println!("Reading all props took: {:.2?}", elapsed);
            sum += elapsed;
        }

        println!("Run average: {:.2?}", sum / 50);
    }
}
