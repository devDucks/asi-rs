use log::{debug, error, info, warn};
use serde::Serialize;
use uuid::Uuid;

use libasi::efw::AsiEfwError;

pub fn look_for_devices() -> i32 {
    let num = libasi::efw::get_num_of_connected_devices();
    match num {
        0 => warn!("No ZWO EFW devices found"),
        n => info!("Found {} ZWO EFW device(s)", n),
    }
    num
}

#[derive(Debug, Serialize)]
pub struct EfwDevice {
    #[serde(skip)]
    pub id: Uuid,
    pub name: String,
    #[serde(skip)]
    efw_id: i32,
    pub slot_num: i32,
    pub current_slot: i32,
    pub unidirectional: bool,
    pub calibrating: bool,
}

impl EfwDevice {
    pub fn new(index: i32) -> Result<Self, AsiEfwError> {
        let mut efw_id: i32 = 0;
        libasi::efw::get_efw_id(index, &mut efw_id)?;

        libasi::efw::open_efw(efw_id)?;

        let mut info = libasi::efw::EFWInfo::new();
        libasi::efw::get_efw_property(efw_id, &mut info)?;

        let name = asi_rs::utils::asi_name_to_string(&info.Name);
        let slot_num = info.slotNum;
        let current_slot = libasi::efw::get_efw_position(efw_id)?;
        let unidirectional = libasi::efw::is_unidirectional(efw_id)?;

        info!(
            "EFW '{}' opened: {} slots, current={}, unidirectional={}",
            name, slot_num, current_slot, unidirectional
        );

        Ok(Self {
            id: Uuid::new_v4(),
            name: format!("ZWO {}", name),
            efw_id,
            slot_num,
            current_slot,
            unidirectional,
            calibrating: false,
        })
    }

    pub fn fetch_props(&mut self) {
        // Don't poll while calibrating — position will be -1 (returns 0 via wrapper)
        if self.calibrating {
            return;
        }
        match libasi::efw::get_efw_position(self.efw_id) {
            Ok(slot) => {
                if self.current_slot != slot {
                    debug!("Slot changed: {} -> {}", self.current_slot, slot);
                    self.current_slot = slot;
                }
            }
            Err(e) => error!("Failed to get EFW position: {e}"),
        }
        match libasi::efw::is_unidirectional(self.efw_id) {
            Ok(unid) => {
                if self.unidirectional != unid {
                    self.unidirectional = unid;
                }
            }
            Err(e) => error!("Failed to get EFW direction: {e}"),
        }
    }

    pub fn set_slot(&self, position: i32) {
        debug!("Setting EFW slot to {}", position);
        if let Err(e) = libasi::efw::set_efw_position(self.efw_id, position) {
            error!("Failed to set EFW slot {position}: {e}");
        }
    }

    pub fn set_unidirectional(&self, flag: bool) {
        debug!("Setting EFW unidirectional to {}", flag);
        if let Err(e) = libasi::efw::set_unidirection(self.efw_id, flag) {
            error!("Failed to set EFW unidirectional: {e}");
        }
    }

    pub fn efw_id(&self) -> i32 {
        self.efw_id
    }

    pub fn close(&self) {
        debug!("Closing EFW '{}'", self.name);
        let _ = libasi::efw::close_efw(self.efw_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn efw_device_skips_internal_fields() {
        let device = EfwDevice {
            id: Uuid::new_v4(),
            name: "ZWO EFW Mini".to_string(),
            efw_id: 42,
            slot_num: 5,
            current_slot: 2,
            unidirectional: false,
            calibrating: false,
        };
        let json = serde_json::to_string(&device).unwrap();
        // Internal fields must not appear in serialized output
        assert!(!json.contains("\"id\""));
        assert!(!json.contains("\"efw_id\""));
        // Public fields must be present
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"slot_num\""));
        assert!(json.contains("\"current_slot\""));
        assert!(json.contains("\"unidirectional\""));
        assert!(json.contains("\"calibrating\""));
    }
}
