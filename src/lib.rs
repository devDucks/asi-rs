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
        let result = asi_name_to_string(&name);
        // 'A' + '#' + 'B' = "A#B"
        assert_eq!(result, "A#B");
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
