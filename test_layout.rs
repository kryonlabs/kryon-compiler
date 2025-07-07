// Simple test for layout parsing
use kryc::utils::parse_layout_string;
use kryc::types::{LAYOUT_DIRECTION_ABSOLUTE, LAYOUT_DIRECTION_ROW, LAYOUT_DIRECTION_COLUMN, LAYOUT_ALIGNMENT_CENTER};

fn main() {
    println!("Testing layout parsing...");
    
    // Test absolute layout
    match parse_layout_string("absolute") {
        Ok(flags) => {
            println!("Absolute layout: 0x{:02X} (binary: {:08b})", flags, flags);
            let direction = flags & 0x03;
            println!("Direction bits: 0x{:02X}", direction);
            if direction == LAYOUT_DIRECTION_ABSOLUTE {
                println!("✓ Absolute layout correctly parsed as direction {}", LAYOUT_DIRECTION_ABSOLUTE);
            } else {
                println!("✗ Absolute layout incorrectly parsed as direction {}", direction);
            }
        }
        Err(e) => println!("✗ Failed to parse absolute layout: {}", e),
    }
    
    // Test absolute with alignment
    match parse_layout_string("absolute center") {
        Ok(flags) => {
            println!("Absolute center layout: 0x{:02X} (binary: {:08b})", flags, flags);
            let direction = flags & 0x03;
            let alignment = (flags >> 2) & 0x03;
            println!("Direction: {}, Alignment: {}", direction, alignment);
        }
        Err(e) => println!("✗ Failed to parse absolute center layout: {}", e),
    }
    
    // Test other layouts for comparison
    match parse_layout_string("row") {
        Ok(flags) => println!("Row layout: 0x{:02X}", flags),
        Err(e) => println!("✗ Failed to parse row layout: {}", e),
    }
    
    match parse_layout_string("column") {
        Ok(flags) => println!("Column layout: 0x{:02X}", flags),
        Err(e) => println!("✗ Failed to parse column layout: {}", e),
    }
}