pub mod utils {
    pub fn new_read_only_prop(
        name: &str,
        value: &str,
        kind: &str,
    ) -> lightspeed_astro::props::Property {
        lightspeed_astro::props::Property {
            name: name.to_string(),
            value: value.to_string(),
            kind: kind.to_string(),
            permission: 0,
        }
    }

    pub fn new_read_write_prop(
        name: &str,
        value: &str,
        kind: &str,
    ) -> lightspeed_astro::props::Property {
        lightspeed_astro::props::Property {
            name: name.to_string(),
            value: value.to_string(),
            kind: kind.to_string(),
            permission: 1,
        }
    }

    /// Converts a null-terminated C-style `i8` array (as returned by the ASI SDK) into
    /// a Rust `String`. Non-representable bytes are replaced with `#`.
    pub fn asi_name_to_string(name_array: &[i8]) -> String {
        let mut to_u8: Vec<u8> = vec![];

        for el in name_array {
            if *el == 0 {
                break;
            }
            match (*el).try_into() {
                Ok(v) => to_u8.push(v),
                Err(_) => to_u8.push(0x23), // '#'
            }
        }
        std::str::from_utf8(&to_u8)
            .map(str::to_string)
            .unwrap_or_else(|_| String::from("UNKNOWN"))
    }

    /// Converts a null-terminated `u8` ID array (as returned by the ASI SDK) into a
    /// Rust `String`.
    pub fn asi_id_to_string(id_array: &[u8]) -> String {
        let len = id_array.iter().position(|&b| b == 0).unwrap_or(id_array.len());
        std::str::from_utf8(&id_array[..len])
            .map(str::to_string)
            .unwrap_or_else(|_| String::from("UNKNOWN"))
    }
}

#[cfg(test)]
mod tests {
    use super::utils::*;

    // --- asi_name_to_string tests ---

    #[test]
    fn test_asi_name_to_string_normal_ascii() {
        // "ZWO" encoded as i8, null-terminated
        let name: Vec<i8> = vec![90, 87, 79, 0, 0, 0];
        assert_eq!(asi_name_to_string(&name), "ZWO");
    }

    #[test]
    fn test_asi_name_to_string_stops_at_null() {
        // "AB\0CD" — should stop at the null and return "AB"
        let name: Vec<i8> = vec![65, 66, 0, 67, 68];
        assert_eq!(asi_name_to_string(&name), "AB");
    }

    #[test]
    fn test_asi_name_to_string_empty_array() {
        let name: Vec<i8> = vec![0, 0, 0];
        assert_eq!(asi_name_to_string(&name), "");
    }

    #[test]
    fn test_asi_name_to_string_all_chars_before_null() {
        // "ASI" with no trailing null — entire array is used
        let name: Vec<i8> = vec![65, 83, 73];
        assert_eq!(asi_name_to_string(&name), "ASI");
    }

    #[test]
    fn test_asi_name_to_string_negative_i8_replaced_with_hash() {
        // i8 value -1 cannot convert to u8, gets replaced with '#' (0x23 = 35)
        let name: Vec<i8> = vec![65, -1, 66, 0];
        assert_eq!(asi_name_to_string(&name), "A#B");
    }

    // --- asi_id_to_string tests ---

    #[test]
    fn test_asi_id_to_string_normal_ascii() {
        let id: Vec<u8> = vec![65, 66, 67, 0, 0];
        assert_eq!(asi_id_to_string(&id), "ABC");
    }

    #[test]
    fn test_asi_id_to_string_stops_at_null() {
        let id: Vec<u8> = vec![65, 0, 66, 67];
        assert_eq!(asi_id_to_string(&id), "A");
    }

    #[test]
    fn test_asi_id_to_string_all_zeros_returns_empty() {
        let id: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(asi_id_to_string(&id), "");
    }

    #[test]
    fn test_asi_id_to_string_full_array_no_null() {
        let id: Vec<u8> = vec![65, 83, 73, 67, 65, 77, 49, 50];
        assert_eq!(asi_id_to_string(&id), "ASICAM12");
    }
}
