use std::sync::{Arc, RwLock};
use std::time::Duration;

use env_logger::Env;
use log::{debug, info};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use tokio::signal;
use tokio::task;
use uuid::Uuid;

pub mod ccd;
use ccd::utils;
use ccd::AsiCamera;
use std::time::Instant;

use rumqttc::Event::{Incoming, Outgoing};
use rumqttc::Packet::Publish;

#[derive(Default, Clone)]
struct AsiCcd {
    devices: Vec<Arc<RwLock<AsiCamera>>>,
}

impl AsiCcd {
    fn new() -> Self {
        let found = utils::look_for_devices();
        let mut devices: Vec<Arc<RwLock<AsiCamera>>> = Vec::with_capacity(found as usize);

        for idx in 0..found {
            let device = Arc::new(RwLock::new(AsiCamera::new(idx)));
            devices.push(device)
        }

        Self { devices }
    }
}

async fn subscribe(client: AsyncClient, ids: &Vec<Uuid>) {
    for id in ids {
        client
            .subscribe(
                format!("{}", format_args!("devices/{}/expose", &id)),
                QoS::AtLeastOnce,
            )
            .await
            .unwrap();
        client
            .subscribe(
                format!("{}", format_args!("devices/{}/update", &id)),
                QoS::AtLeastOnce,
            )
            .await
            .unwrap();
    }
}

#[tokio::main]
async fn main() {
    console_subscriber::init();
    let env = Env::default().filter_or("LS_LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    let driver = AsiCcd::new();
    let mut mqttoptions = MqttOptions::new("asi_ccd", "127.0.0.1", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    let mut devices_id = Vec::with_capacity(driver.devices.len());

    for d in &driver.devices {
        devices_id.push(d.read().unwrap().id)
    }

    subscribe(client.clone(), &devices_id).await;

    for d in &driver.devices {
        let device = Arc::clone(d);

        tokio::spawn(async move {
            signal::ctrl_c().await.unwrap();
            debug!("ctrl-c received!");
            device.read().unwrap().close();
            std::process::exit(0);
        });
    }

    for d in &driver.devices {
        let device = Arc::clone(d);
        let c = client.clone();
        task::spawn(async move {
            let d_id = device.read().unwrap().id;
            loop {
                let now = Instant::now();
                device.write().unwrap().fetch_props();
                let serialized = serde_json::to_string(&*device.read().unwrap()).unwrap();
                c.publish(
                    format!("{}", format_args!("devices/{}", &d_id)),
                    QoS::AtLeastOnce,
                    false,
                    serialized,
                )
                .await
                .unwrap();
                let elapsed = now.elapsed();
                debug!("Refreshed and publishing state took: {:.2?}", elapsed);
                tokio::time::sleep(Duration::from_millis(2500)).await;
            }
        });
    }

    while let Ok(event) = eventloop.poll().await {
        debug!("Received = {:?}", event);
        match event {
            Incoming(inc) => match inc {
                Publish(data) => {
                    // All topics are in the form of devices/{UUID}/{action} so let's
                    // take advantage of this fact and avoid a string split
                    match &data.topic[45..data.topic.len()] {
                        "update" => {
                            info!(
                                "received message from topic: {}\nmessage: {:?}",
                                &data.topic, &data.payload
                            );
                            let device = &data.topic[8..44];
                            for d in &driver.devices {
                                if device == &d.read().unwrap().id.to_string() {
                                    d.write().unwrap().update_property("img_type", 1)
                                }
                            }
                        }
                        "expose" => {
                            let device = &data.topic[8..44];
                            info!("mqtt id: `{}`", &device);

                            for d in &driver.devices {
                                info!("device id: `{}`", &d.read().unwrap().id.to_string());
                                if device == &d.read().unwrap().id.to_string() {
                                    let device = Arc::clone(d);
                                    let _c = client.clone();
                                    task::spawn_blocking(move || {
                                        utils::capturing::expose(
                                            2.0,
                                            libasi::camera::ASI_IMG_TYPE_ASI_IMG_RGB24,
                                            device,
                                        );
                                        info!("Task ended");
                                    });
                                }
                            }
                        }
                        _ => (),
                    }
                }
                _ => debug!("Incoming event: {:?}", inc),
            },
            Outgoing(out) => {
                debug!("Outgoing MQTT event: {:?}", out);
            }
        }
    }
}
