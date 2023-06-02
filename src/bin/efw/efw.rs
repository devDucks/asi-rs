use std::str::FromStr;
use std::sync::{Arc, RwLock};

use lightspeed_astro::devices::actions::DeviceActions;
use lightspeed_astro::props::Property;
use log::{debug, info, warn};
use uuid::Uuid;

const CALIBRATION_OFF: &str = "off";
const CALIBRATION_ON: &str = "on";

pub fn look_for_devices() -> i32 {
    let num_of_devs = libasi::efw::get_num_of_connected_devices();

    match num_of_devs {
        0 => warn!("No ZWO EFW found"),
        _ => info!("Found {} ZWO EFW(s)", num_of_devs),
    }
    num_of_devs
}

pub fn calibrate(camera_index: i32, device: Arc<RwLock<EfwDevice>>) {
    debug!("Calibrating wheel");

    {
        let mut d = device.write().unwrap();
        d.update_internal_property("calibration", CALIBRATION_ON);
    }

    libasi::efw::calibrate_wheel(camera_index);

    while libasi::efw::check_wheel_is_moving(camera_index) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    {
        let mut d = device.write().unwrap();
        d.update_internal_property("calibration", CALIBRATION_OFF);
    }
}

pub trait BaseAstroDevice {
    /// Main and only entrypoint to create a new serial device.
    ///
    /// A device that doesn't work/cannot communicate with is not really useful
    /// so this may return `None` if there is something wrong with the just
    /// discovered device.
    fn new(index: i32) -> Self
    where
        Self: Sized;

    /// Use this method to fetch the real properties from the device,
    /// this should not be called directly from clients ideally,
    /// for that goal `get_properties` should be used.
    fn fetch_props(&mut self);

    /// Use this method to return the id of the device as a uuid.
    fn get_id(&self) -> Uuid;

    /// Use this method to return the name of the device (e.g. ZWO533MC).
    fn get_name(&self) -> &String;

    /// Use this method to return the actual cached state stored into `self.properties`.
    fn get_properties(&self) -> &Vec<Property>;

    /// Method to be used when receving requests from clients to update properties.
    ///
    /// Ideally this should call internally `update_property_remote` which will be
    /// responsible to trigger the action against the device to update the property
    /// on the device itself, if the action is successful the last thing this method
    /// does would be to update the property inside `self.properties`.
    fn update_property(&mut self, prop_name: &str, val: &str) -> Result<(), DeviceActions>;

    /// Method used internally by the driver itself to change values for properties that
    /// be manipulated by the user (like the exposure ones)
    fn update_internal_property(&mut self, prop_name: &str, val: &str)
        -> Result<(), DeviceActions>;

    /// Use this method to send a command to the device to change the requested property.
    ///
    /// Ideally this method will be a big `match` clause where the matching will execute
    /// `self.send_command` to issue a serial command to the device.
    fn update_property_remote(&mut self, prop_name: &str, val: &str) -> Result<(), DeviceActions>;

    /// Properties are packed into a vector so to find them we need to
    /// lookup the index, use this method to do so.
    fn find_property_index(&self, prop_name: &str) -> Option<usize>;
}

pub trait AsiEfw {
    fn init_device(&mut self);
    fn close(&self);
    fn get_index(&self) -> i32;
    fn init_device_props(&mut self);
    fn get_info(&self) -> libasi::efw::EFWInfo;
    fn get_position(&self) -> i32;
    fn set_position(&self, position: i32);
    fn set_unidirection(&self, flag: bool);
    fn is_unidirectional(&self) -> bool;
}

pub struct EfwDevice {
    id: Uuid,
    name: String,
    pub properties: Vec<Property>,
    index: i32,
    ls_rand_id: [u8; 8],
    slots_num: i32,
}

impl BaseAstroDevice for EfwDevice {
    fn new(index: i32) -> Self
    where
        Self: Sized,
    {
        let mut efw_id = 0;
        libasi::efw::get_efw_id(index, &mut efw_id);
        let mut device = EfwDevice {
            id: Uuid::new_v4(),
            name: "".to_string(),
            properties: Vec::new(),
            index: efw_id,
            ls_rand_id: [0; 8],
            slots_num: 0,
        };
        device.init_device();
        device.init_device_props();
        device
    }

    fn fetch_props(&mut self) {
        let actual = self.get_position();

        let prop = self.properties.get_mut(1).unwrap();

        if prop.value.parse::<i32>().unwrap() != actual {
            prop.value = actual.to_string();
        };
    }

    fn get_id(&self) -> Uuid {
        self.id
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_properties(&self) -> &Vec<Property> {
        &self.properties
    }

    fn update_property(&mut self, prop_name: &str, val: &str) -> Result<(), DeviceActions> {
        if let Some(idx) = self.find_property_index(prop_name) {
            let prop = self.properties.get(idx).unwrap();

            match prop.permission {
                0 => Err(DeviceActions::CannotUpdateReadOnlyProperty),
                _ => match self.update_property_remote(prop_name, val) {
                    Ok(()) => {
                        let prop = self.properties.get_mut(idx).unwrap();
                        prop.value = val.to_owned();
                        return Ok(());
                    }
                    Err(e) => {
                        info!("Update property remote failed");
                        return Err(e);
                    }
                },
            }
        } else {
            Err(DeviceActions::UnknownProperty)
        }
    }

    fn update_internal_property(
        &mut self,
        prop_name: &str,
        val: &str,
    ) -> Result<(), DeviceActions> {
        info!(
            "driver updating internal property {} with {}",
            prop_name, val
        );
        if let Some(prop_idx) = self.find_property_index(prop_name) {
            let mut prop = self.properties.get_mut(prop_idx).unwrap();
            prop.value = val.to_string();
            Ok(())
        } else {
            Err(DeviceActions::UnknownProperty)
        }
    }

    fn update_property_remote(&mut self, prop_name: &str, val: &str) -> Result<(), DeviceActions> {
        match prop_name {
            "actual_slot" => {
                if let Ok(num) = val.parse::<i32>() {
                    match num {
                        1..=5 => {
                            self.set_position(num);
                            Ok(())
                        }
                        _ => Err(DeviceActions::InvalidValue),
                    }
                } else {
                    Err(DeviceActions::InvalidValue)
                }
            }
            "unidirectional" => {
                if let Ok(flag) = FromStr::from_str(val) {
                    self.set_unidirection(flag);
                    Ok(())
                } else {
                    info!("Parsing failed");
                    Err(DeviceActions::InvalidValue)
                }
            }
            _ => Err(DeviceActions::InvalidValue), // Update to InvalidProperty
        }
    }

    fn find_property_index(&self, prop_name: &str) -> Option<usize> {
        let mut index = 256;

        for (idx, prop) in self.properties.iter().enumerate() {
            if prop.name == prop_name {
                index = idx;
                break;
            }
        }
        if index == 256 {
            return None;
        } else {
            return Some(index);
        }
    }
}

impl AsiEfw for EfwDevice {
    fn get_index(&self) -> i32 {
        self.index
    }

    fn init_device(&mut self) {
        libasi::efw::open_efw(self.index);
        let efw_info = self.get_info();
        debug!("Prop: {:?}", efw_info);
        let name = asi_rs::utils::asi_name_to_string(&efw_info.Name);

        self.name = format!("ZWO {}", name);
        self.slots_num = efw_info.slotNum;
    }

    fn close(&self) {
        libasi::efw::close_efw(self.index);
    }

    fn init_device_props(&mut self) {
        self.properties.push(asi_rs::utils::new_read_only_prop(
            "available_slots",
            &self.slots_num.to_string(),
            "integer",
        ));

        self.properties.push(asi_rs::utils::new_read_write_prop(
            "actual_slot",
            &self.get_position().to_string(),
            "integer",
        ));

        self.properties.push(asi_rs::utils::new_read_write_prop(
            "unidirectional",
            &self.is_unidirectional().to_string(),
            "bool",
        ));

        self.properties.push(asi_rs::utils::new_read_only_prop(
            "calibration",
            CALIBRATION_OFF,
            "string",
        ));
    }

    fn get_info(&self) -> libasi::efw::EFWInfo {
        let mut efw_info = libasi::efw::EFWInfo::new();
        libasi::efw::get_efw_property(self.index, &mut efw_info);
        efw_info
    }

    fn get_position(&self) -> i32 {
        libasi::efw::get_efw_position(self.index)
    }

    fn set_position(&self, position: i32) {
        debug!("Setting position {}", position);
        libasi::efw::set_efw_position(self.index, position);
    }

    fn set_unidirection(&self, flag: bool) {
        debug!("Setting unidirectional state to {}", flag);
        libasi::efw::set_unidirection(self.index, flag);
    }

    fn is_unidirectional(&self) -> bool {
        debug!("Checking unidirectional state");
        libasi::efw::is_unidirectional(self.index)
    }
}
