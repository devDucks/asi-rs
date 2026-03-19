use std::sync::{Arc, RwLock};
use std::time::Duration;

use env_logger::Env;
use log::{debug, info};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use tokio::signal;
use tokio::task;
use uuid::Uuid;

use rumqttc::Event::{Incoming, Outgoing};
use rumqttc::Packet::Publish;

pub mod efw;
use efw::EfwDevice;

#[derive(Default, Clone)]
struct AsiEfwDriver {
    devices: Vec<Arc<RwLock<EfwDevice>>>,
}

impl AsiEfwDriver {
    fn new() -> Self {
        let found = efw::look_for_devices();
        let devices = (0..found)
            .filter_map(|idx| {
                EfwDevice::new(idx)
                    .map_err(|e| log::error!("Failed to initialize EFW device {idx}: {e}"))
                    .ok()
            })
            .map(|d| Arc::new(RwLock::new(d)))
            .collect();
        Self { devices }
    }
}

async fn subscribe(client: AsyncClient, ids: &Vec<Uuid>) {
    for id in ids {
        client
            .subscribe(format!("devices/{}/set_slot", id), QoS::AtLeastOnce)
            .await
            .unwrap();
        client
            .subscribe(format!("devices/{}/calibrate", id), QoS::AtLeastOnce)
            .await
            .unwrap();
        client
            .subscribe(format!("devices/{}/update", id), QoS::AtLeastOnce)
            .await
            .unwrap();
    }
}

#[tokio::main]
async fn main() {
    console_subscriber::init();
    let env = Env::default().filter_or("LS_LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    let driver = AsiEfwDriver::new();
    let mut mqttoptions = MqttOptions::new("asi_efw", "127.0.0.1", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    let mut devices_id = Vec::with_capacity(driver.devices.len());
    for d in &driver.devices {
        devices_id.push(d.read().unwrap().id);
    }

    subscribe(client.clone(), &devices_id).await;

    // Spawn a ctrl-c handler per device to close cleanly on shutdown
    for d in &driver.devices {
        let device = Arc::clone(d);
        tokio::spawn(async move {
            signal::ctrl_c().await.unwrap();
            debug!("ctrl-c received, closing EFW device");
            device.read().unwrap().close();
            std::process::exit(0);
        });
    }

    // Periodic state fetch and publish per device
    for d in &driver.devices {
        let device = Arc::clone(d);
        let c = client.clone();
        task::spawn(async move {
            let d_id = device.read().unwrap().id;
            loop {
                device.write().unwrap().fetch_props();
                let serialized = serde_json::to_string(&*device.read().unwrap()).unwrap();
                c.publish(
                    format!("devices/{}", &d_id),
                    QoS::AtLeastOnce,
                    false,
                    serialized,
                )
                .await
                .unwrap();
                tokio::time::sleep(Duration::from_millis(2500)).await;
            }
        });
    }

    // MQTT event loop
    // Topics are in the form devices/{UUID}/{action}
    // "devices/" = 8 chars, UUID = 36 chars, "/" = 1 char → action starts at index 45
    while let Ok(event) = eventloop.poll().await {
        debug!("Received = {:?}", event);
        match event {
            Incoming(inc) => match inc {
                Publish(data) => {
                    let action = &data.topic[45..];
                    let device_id = &data.topic[8..44];

                    match action {
                        "set_slot" => {
                            let payload = String::from_utf8_lossy(&data.payload);
                            if let Ok(slot) = payload.trim().parse::<i32>() {
                                for d in &driver.devices {
                                    if d.read().unwrap().id.to_string() == device_id {
                                        info!("Setting slot {} for {}", slot, device_id);
                                        d.read().unwrap().set_slot(slot);
                                    }
                                }
                            }
                        }
                        "calibrate" => {
                            for d in &driver.devices {
                                if d.read().unwrap().id.to_string() == device_id {
                                    info!("Starting calibration for {}", device_id);
                                    let device = Arc::clone(d);
                                    task::spawn_blocking(move || {
                                        let efw_id = device.read().unwrap().efw_id();
                                        device.write().unwrap().calibrating = true;
                                        if let Err(e) = libasi::efw::calibrate_wheel(efw_id) {
                                            log::error!("Failed to calibrate EFW {efw_id}: {e}");
                                        }
                                        while libasi::efw::check_wheel_is_moving(efw_id) {
                                            std::thread::sleep(Duration::from_millis(100));
                                        }
                                        device.write().unwrap().calibrating = false;
                                        info!("Calibration complete");
                                    });
                                }
                            }
                        }
                        "update" => {
                            let payload = String::from_utf8_lossy(&data.payload);
                            info!(
                                "Update request for {}: {}",
                                device_id, payload
                            );
                            // TODO: parse and dispatch generic property updates
                        }
                        _ => (),
                    }
                }
                _ => debug!("Incoming event: {:?}", inc),
            },
            Outgoing(out) => debug!("Outgoing MQTT event: {:?}", out),
        }
    }
}
