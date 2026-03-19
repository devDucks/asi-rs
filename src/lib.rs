pub mod utils {
    pub fn asi_name_to_string(name_array: &[i8]) -> String {
        let mut to_u8: Vec<u8> = vec![];

        // format the name dropping 0 from the name array
        for (_, el) in name_array.into_iter().enumerate() {
            if *el == 0 {
                break;
            }
            match (*el).try_into() {
                Ok(v) => to_u8.push(v),
                Err(_) => to_u8.push(0x23),
            }
        }
        if let Ok(id) = std::str::from_utf8(&to_u8) {
            id.to_string()
        } else {
            String::from("UNKNOWN")
        }
    }

    pub fn asi_id_to_string(id_array: &[u8]) -> String {
        let mut index: usize = 0;

        // format the name dropping 0 from the name array
        for (_, el) in id_array.into_iter().enumerate() {
            if *el == 0 {
                break;
            }
            index += 1
        }
        if let Ok(id) = std::str::from_utf8(&id_array[0..index]) {
            id.to_string()
        } else {
            String::from("UNKNOWN")
        }
    }
}

/// Parse an MQTT device topic of the form `devices/{UUID}/{action}`.
///
/// Returns `(uuid, action)` on success, or `None` if the topic is malformed
/// (too short, missing separators, or invalid UUID).
pub fn parse_device_topic(topic: &str) -> Option<(uuid::Uuid, &str)> {
    // "devices/" = 8 chars, UUID = 36 chars, "/" = 1 char → minimum length is 46
    if topic.len() < 46 {
        return None;
    }
    let uuid_str = &topic[8..44];
    let action = &topic[45..];
    let uuid = uuid_str.parse::<uuid::Uuid>().ok()?;
    Some((uuid, action))
}

#[cfg(test)]
mod tests {
    use super::*;
    use utils::{asi_id_to_string, asi_name_to_string};

    // --- asi_name_to_string ---

    #[test]
    fn name_normal_with_null_terminator() {
        let mut arr = [0i8; 64];
        for (i, &b) in b"ASI294MC Pro".iter().enumerate() {
            arr[i] = b as i8;
        }
        assert_eq!(asi_name_to_string(&arr), "ASI294MC Pro");
    }

    #[test]
    fn name_all_zeros_returns_empty() {
        let arr = [0i8; 64];
        assert_eq!(asi_name_to_string(&arr), "");
    }

    #[test]
    fn name_no_null_terminator_uses_full_array() {
        let arr: Vec<i8> = b"ABCD".iter().map(|&b| b as i8).collect();
        assert_eq!(asi_name_to_string(&arr), "ABCD");
    }

    #[test]
    fn name_null_in_middle_truncates() {
        let arr: Vec<i8> = vec![65, 66, 0, 67, 68]; // "AB\0CD"
        assert_eq!(asi_name_to_string(&arr), "AB");
    }

    #[test]
    fn name_negative_i8_replaced_with_hash() {
        // i8 values that cannot be converted to u8 (< 0) become '#' (0x23)
        let arr: Vec<i8> = vec![65, -1, 66]; // 'A', invalid, 'B'
        let result = asi_name_to_string(&arr);
        assert_eq!(result, "A#B");
    }

    // --- asi_id_to_string ---

    #[test]
    fn id_normal_with_null_terminator() {
        let mut arr = [0u8; 8];
        for (i, &b) in b"MYID1234".iter().enumerate() {
            arr[i] = b;
        }
        assert_eq!(asi_id_to_string(&arr), "MYID1234");
    }

    #[test]
    fn id_all_zeros_returns_empty() {
        let arr = [0u8; 8];
        assert_eq!(asi_id_to_string(&arr), "");
    }

    #[test]
    fn id_null_in_middle_truncates() {
        let arr: Vec<u8> = vec![65, 66, 0, 67, 68]; // "AB\0CD"
        assert_eq!(asi_id_to_string(&arr), "AB");
    }

    #[test]
    fn id_no_null_uses_full_array() {
        let arr: Vec<u8> = b"ABCD".to_vec();
        assert_eq!(asi_id_to_string(&arr), "ABCD");
    }

    // --- parse_device_topic ---

    #[test]
    fn parse_valid_expose_topic() {
        let uuid = uuid::Uuid::new_v4();
        let topic = format!("devices/{}/expose", uuid);
        let (parsed_uuid, action) = parse_device_topic(&topic).expect("should parse");
        assert_eq!(parsed_uuid, uuid);
        assert_eq!(action, "expose");
    }

    #[test]
    fn parse_valid_set_slot_topic() {
        let uuid = uuid::Uuid::new_v4();
        let topic = format!("devices/{}/set_slot", uuid);
        let (parsed_uuid, action) = parse_device_topic(&topic).expect("should parse");
        assert_eq!(parsed_uuid, uuid);
        assert_eq!(action, "set_slot");
    }

    #[test]
    fn parse_valid_calibrate_topic() {
        let uuid = uuid::Uuid::new_v4();
        let topic = format!("devices/{}/calibrate", uuid);
        let (_, action) = parse_device_topic(&topic).expect("should parse");
        assert_eq!(action, "calibrate");
    }

    #[test]
    fn parse_rejects_topic_too_short() {
        assert!(parse_device_topic("devices/short").is_none());
        assert!(parse_device_topic("").is_none());
    }

    #[test]
    fn parse_rejects_invalid_uuid() {
        let topic = "devices/not-a-valid-uuid-at-all-here/expose";
        assert!(parse_device_topic(topic).is_none());
    }
}
