use crate::ccd::utils::look_for_devices;
use crate::ccd::AsiCcd;
use lightspeed_astro::devices::actions::DeviceActions;
use lightspeed_astro::devices::ProtoDevice;
use lightspeed_astro::props::{SetPropertyRequest, SetPropertyResponse};
use lightspeed_astro::request::{CcdExposureRequest, CcdExposureResponse, GetDevicesRequest};
use lightspeed_astro::response::GetDevicesResponse;
use lightspeed_astro::server::astro_service_server::{AstroService, AstroServiceServer};
use log::{debug, error, info};
use tonic::{transport::Server, Request, Response, Status};
pub mod ccd;
use crate::ccd::AstroDevice;
use ccd::CcdDevice;
use env_logger::Env;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::task;

#[derive(Default, Clone)]
struct AsiCcdDriver {
    devices: Arc<Vec<RwLock<CcdDevice>>>,
}

impl AsiCcdDriver {
    fn new() -> Self {
        let found = look_for_devices();
        let mut devices: Vec<RwLock<CcdDevice>> = Vec::with_capacity(found as usize);
        for dev in 0..found {
            debug!("Trying to create a new device for index {}", dev);
            let device = RwLock::new(CcdDevice::new(dev));
            devices.push(device)
        }
        Self {
            devices: Arc::new(devices),
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

        if self.devices.is_empty() {
            let reply = GetDevicesResponse { devices: vec![] };
            Ok(Response::new(reply))
        } else {
            let mut devices = Vec::new();
            for dev in self.devices.iter() {
		let device = dev.read().unwrap();
		let d = ProtoDevice {
                    id: device.get_id().to_string(),
                    name: device.get_name().to_owned(),
                    family: 0,
                    properties: device.properties.to_owned(),
                };
                devices.push(d);
            }
            let reply = GetDevicesResponse { devices: devices };
            Ok(Response::new(reply))
        }
    }

    async fn expose(
        &self,
        request: Request<CcdExposureRequest>,
    ) -> Result<Response<CcdExposureResponse>, Status> {
        info!("Asking to expose");
        let message = request.get_ref();
        let length = message.lenght.clone();
        let dev_id = message.id.clone();
        let devices = self.devices.clone();
        task::spawn_blocking(move || {
            for d in devices.iter() {
		let mut device = d.write().unwrap();
		if device.get_id().to_string() == dev_id {
		    info!("Task dispatched");
                    device.expose(length);
                }
            }
        });
        
        let reply = CcdExposureResponse { data: vec![] };
        Ok(Response::new(reply))
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
        for d in self.devices.iter() {
	    let mut device = d.write().unwrap();
            if device.get_id().to_string() == message.device_id {
                info!(
                    "Updating property {} for {} to {}",
                    message.property_name, message.device_id, message.property_value,
                );

                if let Err(e) = device.update_property(&message.property_name, &message.property_value) {
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
            for d in devices_for_fetching.iter() {
		let mut device = d.write().unwrap();
                device.fetch_props();
            }
        }
    });

    info!("ZWOASIDriver process listening on {}", addr);
    Server::builder()
        .add_service(reflection_service)
        .add_service(AstroServiceServer::new(driver))
        .serve_with_shutdown(addr, async move {
            match tokio::signal::ctrl_c().await {
                Ok(_) => {
                    debug!("shutting down requested, closing devices before quitting...");
                    for d in devices_for_closing.iter() {
			let device = d.write().unwrap();
                        device.close();
                    }
                }
                Err(e) => error!("An error occurred while intercepting the shutdown: {}", e),
            }
        })
        .await?;
    Ok(())
}
