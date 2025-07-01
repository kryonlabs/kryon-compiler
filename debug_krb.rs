use std::env;
use std::fs;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <krb_file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let data = fs::read(filename).expect("Failed to read KRB file");
    
    if data.len() < 54 {
        eprintln!("File too small for valid KRB");
        std::process::exit(1);
    }

    let mut cursor = Cursor::new(&data);
    
    // Skip magic and version
    cursor.set_position(6);
    
    let flags = cursor.read_u16::<LittleEndian>().unwrap();
    let element_count = cursor.read_u16::<LittleEndian>().unwrap();
    let style_count = cursor.read_u16::<LittleEndian>().unwrap();
    let component_count = cursor.read_u16::<LittleEndian>().unwrap();
    let animation_count = cursor.read_u16::<LittleEndian>().unwrap();
    let script_count = cursor.read_u16::<LittleEndian>().unwrap();
    let string_count = cursor.read_u16::<LittleEndian>().unwrap();
    let resource_count = cursor.read_u16::<LittleEndian>().unwrap();
    
    let element_offset = cursor.read_u32::<LittleEndian>().unwrap();
    let style_offset = cursor.read_u32::<LittleEndian>().unwrap();
    let component_offset = cursor.read_u32::<LittleEndian>().unwrap();
    let animation_offset = cursor.read_u32::<LittleEndian>().unwrap();
    let script_offset = cursor.read_u32::<LittleEndian>().unwrap();
    let string_offset = cursor.read_u32::<LittleEndian>().unwrap();
    let resource_offset = cursor.read_u32::<LittleEndian>().unwrap();
    let total_size = cursor.read_u32::<LittleEndian>().unwrap();
    
    println!("KRB File Analysis:");
    println!("  Elements: {}", element_count);
    println!("  Styles: {}", style_count);
    println!("  Components: {}", component_count);
    println!("  Scripts: {}", script_count);
    println!("  Strings: {}", string_count);
    println!("  Resources: {}", resource_count);
    println!("  Element offset: {}", element_offset);
    println!("  String offset: {}", string_offset);
    println!("  Total size: {}", total_size);
    println!();
    
    // Read elements if they exist
    if element_count > 0 {
        println!("Elements:");
        cursor.set_position(element_offset as u64);
        
        for i in 0..element_count {
            let element_type = cursor.read_u8().unwrap();
            let id_string_index = cursor.read_u8().unwrap();
            let pos_x = cursor.read_u16::<LittleEndian>().unwrap();
            let pos_y = cursor.read_u16::<LittleEndian>().unwrap();
            let width = cursor.read_u16::<LittleEndian>().unwrap();
            let height = cursor.read_u16::<LittleEndian>().unwrap();
            let layout = cursor.read_u8().unwrap();
            let style_id = cursor.read_u8().unwrap();
            let property_count = cursor.read_u8().unwrap();
            let child_count = cursor.read_u8().unwrap();
            let event_count = cursor.read_u8().unwrap();
            let animation_count = cursor.read_u8().unwrap();
            let custom_prop_count = cursor.read_u8().unwrap();
            let state_prop_count = cursor.read_u8().unwrap();
            
            let element_type_name = match element_type {
                0x00 => "App",
                0x01 => "Container", 
                0x02 => "Text",
                0x03 => "Image",
                0x04 => "Canvas",
                0x10 => "Button",
                0x11 => "Input",
                0xFE => "InternalComponentUsage",
                0xFF => "Unknown",
                _ => "Custom/Unknown",
            };
            
            println!("  Element {}: type={} ({}), pos=({}, {}), size=({}, {}), properties={}, children={}",
                     i, element_type, element_type_name, pos_x, pos_y, width, height, property_count, child_count);
            
            // Skip all element data: properties, custom properties, state properties, events, child offsets
            
            // Skip standard properties
            for _ in 0..property_count {
                let _prop_id = cursor.read_u8().unwrap();
                let _value_type = cursor.read_u8().unwrap(); 
                let size = cursor.read_u8().unwrap();
                cursor.set_position(cursor.position() + size as u64);
            }
            
            // Skip custom properties  
            for _ in 0..custom_prop_count {
                let _key_index = cursor.read_u8().unwrap();
                let _value_type = cursor.read_u8().unwrap();
                let size = cursor.read_u8().unwrap();
                cursor.set_position(cursor.position() + size as u64);
            }
            
            // Skip state property sets
            for _ in 0..state_prop_count {
                let _state_flags = cursor.read_u8().unwrap();
                let state_property_count = cursor.read_u8().unwrap();
                
                // Skip properties in this state set
                for _ in 0..state_property_count {
                    let _prop_id = cursor.read_u8().unwrap();
                    let _value_type = cursor.read_u8().unwrap();
                    let size = cursor.read_u8().unwrap();
                    cursor.set_position(cursor.position() + size as u64);
                }
            }
            
            // Skip events
            for _ in 0..event_count {
                let _event_type = cursor.read_u8().unwrap();
                let _callback_id = cursor.read_u8().unwrap();
            }
            
            // Skip child offsets (2 bytes each)
            for _ in 0..child_count {
                let _child_offset = cursor.read_u16::<LittleEndian>().unwrap();
            }
        }
    }
    
    // Read strings if they exist
    if string_count > 0 {
        println!("\nStrings:");
        cursor.set_position(string_offset as u64);
        
        for i in 0..string_count {
            let length = cursor.read_u8().unwrap();
            let mut text = vec![0u8; length as usize];
            cursor.read_exact(&mut text).unwrap();
            let text_str = String::from_utf8_lossy(&text);
            println!("  String {}: \"{}\"", i, text_str);
        }
    }
}