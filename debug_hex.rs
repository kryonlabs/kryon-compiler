fn main() {
    println\!("'R'.is_ascii_hexdigit() = {}", 'R'.is_ascii_hexdigit());
    println\!("'r'.is_ascii_hexdigit() = {}", 'r'.is_ascii_hexdigit());
    println\!("'F'.is_ascii_hexdigit() = {}", 'F'.is_ascii_hexdigit());
    println\!("'G'.is_ascii_hexdigit() = {}", 'G'.is_ascii_hexdigit());
    
    let r_char = 'R';
    println\!("Character: '{}', code: {}, is_hex: {}", r_char, r_char as u32, r_char.is_ascii_hexdigit());
}
EOF < /dev/null
