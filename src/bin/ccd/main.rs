use crate::ccd::utils::look_for_devices;
use crate::ccd::AsiCcd;
use lightspeed_astro::devices::actions::DeviceActions;
use lightspeed_astro::devices::ProtoDevice;
use lightspeed_astro::props::{SetPropertyRequest, SetPropertyResponse};
use lightspeed_astro::request::GetDevicesRequest;
use lightspeed_astro::response::GetDevicesResponse;
use lightspeed_astro::server::astro_service_server::{AstroService, AstroServiceServer};
use log::{debug, error, info};
use tonic::{transport::Server, Request, Response, Status};
pub mod ccd;
use astrotools::AstroSerialDevice;
use ccd::CcdDevice;
use env_logger::Env;

use crate::ccd::AstroDevice;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Default, Clone)]
struct AsiCcdDriver {
    devices: Arc<Mutex<Vec<CcdDevice>>>,
}

impl AsiCcdDriver {
    fn new() -> Self {
        let found = look_for_devices();
        let mut devices: Vec<CcdDevice> = Vec::with_capacity(found as usize);
        for dev in 0..found {
            debug!("Trying to create a new device for index {}", dev);
            let device = CcdDevice::new(dev);
            devices.push(device)
        }
        Self {
            devices: Arc::new(Mutex::new(devices)),
        }
    }
}

#[tonic::async_trait]
impl AstroService for AsiCcdDriver {
    async fn get_devices(
        &self,
        request: Request<GetDevicesRequest>,
    ) -> Result<Response<GetDevicesResponse>, Status> {
        debug!(
            "Got a request to query devices from {:?}",
            request.remote_addr()
        );

        if self.devices.lock().unwrap().is_empty() {
            let reply = GetDevicesResponse { devices: vec![] };
            Ok(Response::new(reply))
        } else {
            let mut devices = Vec::new();
            for device in self.devices.lock().unwrap().iter() {
                let d = ProtoDevice {
                    id: device.get_id().to_string(),
                    name: device.get_name().to_owned(),
                    address: "".to_string(),
                    baud: 0,
                    family: 0,
                    properties: device.properties.to_owned(),
                };
                devices.push(d);
            }
            let reply = GetDevicesResponse { devices: devices };
            Ok(Response::new(reply))
        }
    }

    async fn set_property(
        &self,
        request: Request<SetPropertyRequest>,
    ) -> Result<Response<SetPropertyResponse>, Status> {
        info!(
            "Got a request to set a property from {:?}",
            request.remote_addr()
        );
        let message = request.get_ref();
        debug!("device_id: {:?}", message.device_id);

        if message.device_id == "" || message.property_name == "" || message.property_value == "" {
            return Ok(Response::new(SetPropertyResponse {
                status: DeviceActions::InvalidValue as i32,
            }));
        };

        // TODO: return case if no devices match
        for d in self.devices.lock().unwrap().iter_mut() {
            if d.get_id().to_string() == message.device_id {
                info!(
                    "Updating property {} for {} to {}",
                    message.property_name, message.device_id, message.property_value,
                );

                if let Err(e) = d.update_property(&message.property_name, &message.property_value) {
                    info!(
                        "Updating property {} for {} failed with reason: {:?}",
                        message.property_name, message.device_id, e
                    );
                    return Ok(Response::new(SetPropertyResponse { status: e as i32 }));
                }
            }
        }

        let reply = SetPropertyResponse {
            status: DeviceActions::Ok as i32,
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Default to log level INFO if LS_LOG_LEVEL is not set as
    // an env var
    let env = Env::default().filter_or("LS_LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    // Reflection service
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(lightspeed_astro::proto::FD_DESCRIPTOR_SET)
        .build()
        .unwrap();

    let addr = "127.0.0.1:50051".parse().unwrap();
    let driver = AsiCcdDriver::new();

    let devices_for_fetching = Arc::clone(&driver.devices);
    let devices_for_closing = Arc::clone(&driver.devices);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let mut devices_list = devices_for_fetching.lock().unwrap();
            for device in devices_list.iter_mut() {
                device.fetch_props();
            }
        }
    });

    info!("ZWOASIDriver process listening on {}", addr);
    Server::builder()
        .add_service(reflection_service)
        .add_service(AstroServiceServer::new(driver))
        .serve_with_shutdown(addr, async move {
            tokio::signal::ctrl_c().await;
            debug!("shutting down requested, closing devices before quitting...");
            let mut devices_list = devices_for_closing.lock().unwrap();
            for device in devices_list.iter_mut() {
                device.close();
            }
        })
        .await?;
    Ok(())
}
