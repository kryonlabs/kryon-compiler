use std::collections::HashMap;

fn main() {
    // Test the template variable extraction
    let test_value = "$counter_value";
    let variables = extract_template_variables(test_value);
    println!("Test value: '{}' -> Variables: {:?}", test_value, variables);
    
    // Test the full regex
    let test_values = vec![
        "$counter_value",
        "The count is $counter_value",
        "$counter_value items",
        "Hello $name and $age",
        "No variables here",
        "$_private_var",
        "$var123",
    ];
    
    for test_val in test_values {
        let vars = extract_template_variables(test_val);
        println!("'{}' -> {:?}", test_val, vars);
    }
}

fn extract_template_variables(value: &str) -> Vec<String> {
    use regex::Regex;
    
    let re = Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    let mut variables = Vec::new();
    
    for capture in re.captures_iter(value) {
        if let Some(var_name) = capture.get(1) {
            let name = var_name.as_str().to_string();
            if !variables.contains(&name) {
                variables.push(name);
            }
        }
    }
    
    variables
}