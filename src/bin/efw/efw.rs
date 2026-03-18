use log::{debug, info, warn};
use serde::Serialize;
use uuid::Uuid;

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
    pub fn new(index: i32) -> Self {
        let mut efw_id: i32 = 0;
        libasi::efw::get_efw_id(index, &mut efw_id);

        libasi::efw::open_efw(efw_id);

        let mut info = libasi::efw::EFWInfo::new();
        libasi::efw::get_efw_property(efw_id, &mut info);

        let name = asi_rs::utils::asi_name_to_string(&info.Name);
        let slot_num = info.slotNum;
        let current_slot = libasi::efw::get_efw_position(efw_id);
        let unidirectional = libasi::efw::is_unidirectional(efw_id);

        info!(
            "EFW '{}' opened: {} slots, current={}, unidirectional={}",
            name, slot_num, current_slot, unidirectional
        );

        Self {
            id: Uuid::new_v4(),
            name: format!("ZWO {}", name),
            efw_id,
            slot_num,
            current_slot,
            unidirectional,
            calibrating: false,
        }
    }

    pub fn fetch_props(&mut self) {
        // Don't poll while calibrating — position will be -1 (returns 0 via wrapper)
        if self.calibrating {
            return;
        }
        let slot = libasi::efw::get_efw_position(self.efw_id);
        if self.current_slot != slot {
            debug!("Slot changed: {} -> {}", self.current_slot, slot);
            self.current_slot = slot;
        }
        let unid = libasi::efw::is_unidirectional(self.efw_id);
        if self.unidirectional != unid {
            self.unidirectional = unid;
        }
    }

    pub fn set_slot(&self, position: i32) {
        debug!("Setting EFW slot to {}", position);
        libasi::efw::set_efw_position(self.efw_id, position);
    }

    pub fn set_unidirectional(&self, flag: bool) {
        debug!("Setting EFW unidirectional to {}", flag);
        libasi::efw::set_unidirection(self.efw_id, flag);
    }

    pub fn efw_id(&self) -> i32 {
        self.efw_id
    }

    pub fn close(&self) {
        debug!("Closing EFW '{}'", self.name);
        libasi::efw::close_efw(self.efw_id);
    }
}
