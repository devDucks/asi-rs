use crate::ccd::utils::look_for_devices;
use crate::ccd::AsiCcd;
use lightspeed_astro::devices::actions::DeviceActions;
use lightspeed_astro::devices::AstroDevice;
use lightspeed_astro::request::SetPropertyRequest;
use lightspeed_astro::request::{CcdExposureRequest, CcdExposureResponse, GetDevicesRequest};
use lightspeed_astro::response::GetDevicesResponse;
use lightspeed_astro::response::SetPropertyResponse;
use lightspeed_astro::service::astro_ccd_service_server::{AstroCcdService, AstroCcdServiceServer};
use log::{debug, error, info};
use tonic::{transport::Server, Request, Response, Status};
pub mod ccd;
use crate::ccd::BaseAstroDevice;
use ccd::utils;
use ccd::CcdDevice;
use env_logger::Env;
use std::net::TcpListener;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::task;

#[derive(Default, Clone)]
struct AsiCcdDriver {
    devices: Vec<Arc<RwLock<CcdDevice>>>,
}

impl AsiCcdDriver {
    fn new() -> Self {
        let found = look_for_devices();
        let mut devices: Vec<Arc<RwLock<CcdDevice>>> = Vec::with_capacity(found as usize);
        for dev in 0..found {
            debug!("Trying to create a new device for index {}", dev);
            let device = Arc::new(RwLock::new(CcdDevice::new(dev)));
            devices.push(device)
        }
        Self { devices: devices }
    }
}

#[tonic::async_trait]
impl AstroCcdService for AsiCcdDriver {
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
                let d = AstroDevice {
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
        let message = request.get_ref();
        let length = message.lenght.clone();
        let dev_id = message.id.clone();
        let devices = self.devices.clone();

        for d in devices.iter() {
            {
                let device = d.read().unwrap();
                let index = device.get_index().clone();
                let length = length.clone();
                let width = device.get_actual_width();
                let height = device.get_actual_height();
                let img_type = device.get_actual_raw_img_type();
                let d2 = Arc::clone(d);

                if device.get_id().to_string() == dev_id {
                    info!("Task dispatching in a new thread...");
                    task::spawn_blocking(move || {
                        utils::capturing::expose(index, length, width, height, img_type, d2);
                        info!("Task ended");
                    });
                }
            }
        }

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

                if let Err(e) =
                    device.update_property(&message.property_name, &message.property_value)
                {
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

fn port_is_available(host: &str, port: u16) -> bool {
    match TcpListener::bind((host, port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}
fn build_server_address(host: &str) -> std::net::SocketAddr {
    let port = {
        (50051..50651)
            .find(|port| port_is_available(host, *port))
            .unwrap()
    };
    format!("{host}:{port}").parse().unwrap()
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

    let host = "127.0.0.1";
    let addr = build_server_address(host);
    let driver = AsiCcdDriver::new();

    let mut devices_for_fetching = Vec::new();
    let mut devices_for_closing = Vec::new();
    for d in &driver.devices {
        devices_for_fetching.push(Arc::clone(d));
        devices_for_closing.push(Arc::clone(d));
    }

    for d in &devices_for_fetching {
        let device = Arc::clone(d);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                device.write().unwrap().fetch_props();
            }
        });
    }

    info!("ZWOASIDriver process listening on {}", addr);

    Server::builder()
        .add_service(reflection_service)
        .add_service(AstroCcdServiceServer::new(driver))
        .serve_with_shutdown(addr, async move {
            match tokio::signal::ctrl_c().await {
                Ok(_) => {
                    debug!("shutting down requested, closing devices before quitting...");
                    for d in devices_for_closing.iter_mut() {
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
