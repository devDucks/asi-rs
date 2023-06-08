// pub mod utils {
//     use lightspeed_astro::props::{Permission, Property};

//     pub fn asi_name_to_string(name_array: &[i8]) -> String {
//         let mut to_u8: Vec<u8> = vec![];

//         // format the name dropping 0 from the name array
//         for (_, el) in name_array.into_iter().enumerate() {
//             if *el == 0 {
//                 break;
//             }
//             match (*el).try_into() {
//                 Ok(v) => to_u8.push(v),
//                 Err(_) => to_u8.push(0x23),
//             }
//         }
//         if let Ok(id) = std::str::from_utf8(&to_u8) {
//             id.to_string()
//         } else {
//             String::from("UNKNOWN")
//         }
//     }

//     pub fn asi_id_to_string(id_array: &[u8]) -> String {
//         let mut index: usize = 0;

//         // format the name dropping 0 from the name array
//         for (_, el) in id_array.into_iter().enumerate() {
//             if *el == 0 {
//                 break;
//             }
//             index += 1
//         }
//         if let Ok(id) = std::str::from_utf8(&id_array[0..index]) {
//             id.to_string()
//         } else {
//             String::from("UNKNOWN")
//         }
//     }

//     pub fn new_read_only_prop(name: &str, value: &str, kind: &str) -> Property {
//         Property {
//             name: name.to_string(),
//             value: value.to_string(),
//             kind: kind.to_string(),
//             permission: Permission::ReadOnly as i32,
//         }
//     }

//     pub fn new_read_write_prop(name: &str, value: &str, kind: &str) -> Property {
//         Property {
//             name: name.to_string(),
//             value: value.to_string(),
//             kind: kind.to_string(),
//             permission: Permission::ReadWrite as i32,
//         }
//     }
// }
