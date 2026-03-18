use std::str::FromStr;
use std::sync::{Arc, RwLock};

use libasi::efw::{EFWInfo, EfwHardware};
use lightspeed_astro::devices::actions::DeviceActions;
use lightspeed_astro::props::Property;
use log::{debug, info, warn};
use uuid::Uuid;

const CALIBRATION_OFF: &str = "off";
const CALIBRATION_ON: &str = "on";

pub fn look_for_devices(hw: &dyn EfwHardware) -> i32 {
    let num_of_devs = hw.get_num_of_connected_devices();
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
        d.update_internal_property("calibration", CALIBRATION_ON).ok();
    }

    let hw = {
        let d = device.read().unwrap();
        Arc::clone(&d.hw)
    };

    hw.calibrate(camera_index)
        .unwrap_or_else(|e| log::error!("calibrate failed: {:?}", e));

    while hw.is_moving(camera_index) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    {
        let mut d = device.write().unwrap();
        d.update_internal_property("calibration", CALIBRATION_OFF).ok();
    }
}

pub trait BaseAstroDevice {
    fn new(index: i32) -> Self
    where
        Self: Sized;

    fn fetch_props(&mut self);
    fn get_id(&self) -> Uuid;
    fn get_name(&self) -> &String;
    fn get_properties(&self) -> &Vec<Property>;
    fn update_property(&mut self, prop_name: &str, val: &str) -> Result<(), DeviceActions>;
    fn update_internal_property(
        &mut self,
        prop_name: &str,
        val: &str,
    ) -> Result<(), DeviceActions>;
    fn update_property_remote(
        &mut self,
        prop_name: &str,
        val: &str,
    ) -> Result<(), DeviceActions>;
    fn find_property_index(&self, prop_name: &str) -> Option<usize>;
}

pub trait AsiEfw {
    fn init_device(&mut self);
    fn close(&self);
    fn get_index(&self) -> i32;
    fn init_device_props(&mut self);
    fn get_info(&self) -> EFWInfo;
}

pub struct EfwDevice {
    id: Uuid,
    name: String,
    pub properties: Vec<Property>,
    index: i32,
    ls_rand_id: [u8; 8],
    slots_num: i32,
    pub hw: Arc<dyn EfwHardware>,
}

impl EfwDevice {
    /// Create a new `EfwDevice` with an injected hardware implementation.
    /// Use this in tests to pass a mock, or via `BaseAstroDevice::new` for real hardware.
    pub fn new_with_hw(index: i32, hw: Arc<dyn EfwHardware>) -> Self {
        let efw_id = hw
            .get_id(index)
            .unwrap_or_else(|e| {
                log::error!("get_id failed: {:?}", e);
                0
            });
        let mut device = EfwDevice {
            id: Uuid::new_v4(),
            name: "".to_string(),
            properties: Vec::new(),
            index: efw_id,
            ls_rand_id: [0; 8],
            slots_num: 0,
            hw,
        };
        device.init_device();
        device.init_device_props();
        device
    }
}

impl BaseAstroDevice for EfwDevice {
    fn new(index: i32) -> Self {
        Self::new_with_hw(index, Arc::new(libasi::efw::RealEfw))
    }

    fn fetch_props(&mut self) {
        let actual = self
            .hw
            .get_position(self.index)
            .unwrap_or_else(|e| {
                log::error!("get_position failed: {:?}", e);
                0
            });

        let prop = self.properties.get_mut(1).unwrap();
        if prop.value.parse::<i32>().unwrap() != actual {
            prop.value = actual.to_string();
        }
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
                        Ok(())
                    }
                    Err(e) => {
                        info!("Update property remote failed");
                        Err(e)
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
            let prop = self.properties.get_mut(prop_idx).unwrap();
            prop.value = val.to_string();
            Ok(())
        } else {
            Err(DeviceActions::UnknownProperty)
        }
    }

    fn update_property_remote(
        &mut self,
        prop_name: &str,
        val: &str,
    ) -> Result<(), DeviceActions> {
        match prop_name {
            "actual_slot" => {
                if let Ok(num) = val.parse::<i32>() {
                    if (1..=self.slots_num).contains(&num) {
                        self.hw
                            .set_position(self.index, num)
                            .map_err(|_| DeviceActions::InvalidValue)
                    } else {
                        Err(DeviceActions::InvalidValue)
                    }
                } else {
                    Err(DeviceActions::InvalidValue)
                }
            }
            "unidirectional" => {
                if let Ok(flag) = FromStr::from_str(val) {
                    self.hw
                        .set_direction(self.index, flag)
                        .map_err(|_| DeviceActions::InvalidValue)
                } else {
                    info!("Parsing failed");
                    Err(DeviceActions::InvalidValue)
                }
            }
            _ => Err(DeviceActions::InvalidValue),
        }
    }

    fn find_property_index(&self, prop_name: &str) -> Option<usize> {
        self.properties
            .iter()
            .position(|prop| prop.name == prop_name)
    }
}

impl AsiEfw for EfwDevice {
    fn get_index(&self) -> i32 {
        self.index
    }

    fn init_device(&mut self) {
        self.hw
            .open(self.index)
            .unwrap_or_else(|e| log::error!("open_efw failed: {:?}", e));
        let efw_info = self.get_info();
        debug!("Prop: {:?}", efw_info);
        let name = asi_rs::utils::asi_name_to_string(&efw_info.Name);
        self.name = format!("ZWO {}", name);
        self.slots_num = efw_info.slotNum;
    }

    fn close(&self) {
        self.hw
            .close(self.index)
            .unwrap_or_else(|e| log::error!("close_efw failed: {:?}", e));
    }

    fn init_device_props(&mut self) {
        self.properties.push(asi_rs::utils::new_read_only_prop(
            "available_slots",
            &self.slots_num.to_string(),
            "integer",
        ));

        let actual = self
            .hw
            .get_position(self.index)
            .unwrap_or_else(|e| {
                log::error!("get_position failed: {:?}", e);
                1
            });
        self.properties.push(asi_rs::utils::new_read_write_prop(
            "actual_slot",
            &actual.to_string(),
            "integer",
        ));

        let unid = self
            .hw
            .get_direction(self.index)
            .unwrap_or_else(|e| {
                log::error!("get_direction failed: {:?}", e);
                false
            });
        self.properties.push(asi_rs::utils::new_read_write_prop(
            "unidirectional",
            &unid.to_string(),
            "bool",
        ));

        self.properties.push(asi_rs::utils::new_read_only_prop(
            "calibration",
            CALIBRATION_OFF,
            "string",
        ));
    }

    fn get_info(&self) -> EFWInfo {
        self.hw
            .get_property(self.index)
            .unwrap_or_else(|e| {
                log::error!("get_efw_property failed: {:?}", e);
                EFWInfo::new()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libasi::efw::{EFWInfo, EfwError, EfwHardware};
    use lightspeed_astro::devices::actions::DeviceActions;
    use std::sync::{Arc, Mutex};

    // -----------------------------------------------------------------------
    // Mock EFW hardware
    // -----------------------------------------------------------------------

    struct MockEfw {
        position: Mutex<i32>,
        unidirectional: Mutex<bool>,
        slots: i32,
    }

    impl MockEfw {
        fn new(slots: i32) -> Self {
            MockEfw {
                position: Mutex::new(1),
                unidirectional: Mutex::new(false),
                slots,
            }
        }
    }

    impl EfwHardware for MockEfw {
        fn get_num_of_connected_devices(&self) -> i32 {
            1
        }
        fn get_id(&self, _index: i32) -> Result<i32, EfwError> {
            Ok(0)
        }
        fn open(&self, _id: i32) -> Result<(), EfwError> {
            Ok(())
        }
        fn close(&self, _id: i32) -> Result<(), EfwError> {
            Ok(())
        }
        fn get_property(&self, _id: i32) -> Result<EFWInfo, EfwError> {
            let mut info = EFWInfo::new();
            let name = b"EFW8";
            for (i, b) in name.iter().enumerate() {
                info.Name[i] = *b as i8;
            }
            info.slotNum = self.slots;
            Ok(info)
        }
        fn get_position(&self, _id: i32) -> Result<i32, EfwError> {
            Ok(*self.position.lock().unwrap())
        }
        fn set_position(&self, _id: i32, position: i32) -> Result<(), EfwError> {
            *self.position.lock().unwrap() = position;
            Ok(())
        }
        fn set_direction(&self, _id: i32, flag: bool) -> Result<(), EfwError> {
            *self.unidirectional.lock().unwrap() = flag;
            Ok(())
        }
        fn get_direction(&self, _id: i32) -> Result<bool, EfwError> {
            Ok(*self.unidirectional.lock().unwrap())
        }
        fn calibrate(&self, _id: i32) -> Result<(), EfwError> {
            Ok(())
        }
        fn is_moving(&self, _id: i32) -> bool {
            false
        }
    }

    fn make_device(slots: i32) -> EfwDevice {
        EfwDevice::new_with_hw(0, Arc::new(MockEfw::new(slots)))
    }

    // -----------------------------------------------------------------------
    // find_property_index tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_find_property_index_first_prop() {
        let device = make_device(5);
        assert_eq!(device.find_property_index("available_slots"), Some(0));
    }

    #[test]
    fn test_find_property_index_middle_prop() {
        let device = make_device(5);
        assert_eq!(device.find_property_index("actual_slot"), Some(1));
    }

    #[test]
    fn test_find_property_index_last_prop() {
        let device = make_device(5);
        assert_eq!(device.find_property_index("calibration"), Some(3));
    }

    #[test]
    fn test_find_property_index_not_found() {
        let device = make_device(5);
        assert_eq!(device.find_property_index("nonexistent"), None);
    }

    // -----------------------------------------------------------------------
    // update_property permission tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_update_readonly_property_fails() {
        let mut device = make_device(5);
        let result = device.update_property("available_slots", "10");
        assert_eq!(result, Err(DeviceActions::CannotUpdateReadOnlyProperty));
    }

    #[test]
    fn test_update_unknown_property_fails() {
        let mut device = make_device(5);
        let result = device.update_property("no_such_prop", "1");
        assert_eq!(result, Err(DeviceActions::UnknownProperty));
    }

    // -----------------------------------------------------------------------
    // update_property_remote routing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_valid_slot_succeeds() {
        let mut device = make_device(5);
        assert!(device.update_property("actual_slot", "3").is_ok());
        // Verify the hw mock received the position
        let pos = device.hw.get_position(0).unwrap();
        assert_eq!(pos, 3);
    }

    #[test]
    fn test_set_slot_out_of_range_fails() {
        let mut device = make_device(5);
        // Slot 6 exceeds slots_num of 5
        assert_eq!(
            device.update_property("actual_slot", "6"),
            Err(DeviceActions::InvalidValue)
        );
        // Slot 0 is below the valid range
        assert_eq!(
            device.update_property("actual_slot", "0"),
            Err(DeviceActions::InvalidValue)
        );
    }

    #[test]
    fn test_set_slot_respects_actual_slot_count() {
        // A wheel with only 3 slots should reject slot 4
        let mut device = make_device(3);
        assert!(device.update_property("actual_slot", "3").is_ok());
        assert_eq!(
            device.update_property("actual_slot", "4"),
            Err(DeviceActions::InvalidValue)
        );
    }

    #[test]
    fn test_set_slot_non_numeric_fails() {
        let mut device = make_device(5);
        assert_eq!(
            device.update_property("actual_slot", "abc"),
            Err(DeviceActions::InvalidValue)
        );
    }

    #[test]
    fn test_set_unidirectional_true() {
        let mut device = make_device(5);
        assert!(device.update_property("unidirectional", "true").is_ok());
        assert!(device.hw.get_direction(0).unwrap());
    }

    #[test]
    fn test_set_unidirectional_false() {
        let mut device = make_device(5);
        // First set to true, then back to false
        device.update_property("unidirectional", "true").unwrap();
        assert!(device.update_property("unidirectional", "false").is_ok());
        assert!(!device.hw.get_direction(0).unwrap());
    }

    #[test]
    fn test_set_unidirectional_invalid_value_fails() {
        let mut device = make_device(5);
        assert_eq!(
            device.update_property("unidirectional", "yes"),
            Err(DeviceActions::InvalidValue)
        );
    }

    // -----------------------------------------------------------------------
    // update_internal_property tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_update_internal_property_calibration() {
        let mut device = make_device(5);
        assert!(device
            .update_internal_property("calibration", "on")
            .is_ok());
        let idx = device.find_property_index("calibration").unwrap();
        assert_eq!(device.properties[idx].value, "on");
    }

    #[test]
    fn test_update_internal_property_unknown_fails() {
        let mut device = make_device(5);
        assert_eq!(
            device.update_internal_property("no_such_prop", "val"),
            Err(DeviceActions::UnknownProperty)
        );
    }
}
