//! Utility functions for the Kryon compiler

use crate::error::{CompilerError, Result};
use crate::types::{Color, ValueType, VariableDef, CompilerState};

use meval;
use regex::Regex;
use std::collections::{HashMap, HashSet};


/// Variable processor for handling @variables blocks
pub struct VariableProcessor {
    var_usage_regex: Regex,
}


impl VariableProcessor {
    pub fn new() -> Self {
        Self {
            var_usage_regex: Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
        }
    }

    /// Process variables in source: collect, resolve, and substitute
    pub fn process_and_substitute_variables(
        &self,
        source: &str,
        state: &mut CompilerState,
    ) -> Result<String> {
        // Clear any existing variables
        state.variables.clear();

        // Phase 1: Collect raw variables
        self.collect_raw_variables(source, state)?;
        
        // Phase 2: Resolve inter-variable dependencies
        self.resolve_all_variables(state)?;
        
        // Phase 3: Substitute variables and remove @variables blocks
        self.perform_substitution_and_remove_blocks(source, state)
    }

    /// Scan source for @variables blocks and populate state.variables
    fn collect_raw_variables(&self, source: &str, state: &mut CompilerState) -> Result<()> {
        let mut in_variables_block = false;
        let mut line_num = 0;

        for line in source.lines() {
            line_num += 1;
            let trimmed = line.trim();

            // Skip full-line comments
            if trimmed.starts_with('#') {
                continue;
            }

            // Remove trailing comments
            let line_without_comment = self.strip_trailing_comment(trimmed);

            if line_without_comment == "@variables {" {
                if in_variables_block {
                    return Err(CompilerError::variable(
                        line_num,
                        "Nested @variables blocks are not allowed"
                    ));
                }
                in_variables_block = true;
                continue;
            }

            if line_without_comment == "}" && in_variables_block {
                in_variables_block = false;
                continue;
            }

            if in_variables_block && !line_without_comment.is_empty() {
                self.parse_variable_definition(line_without_comment, line_num, state)?;
            }
        }

        Ok(())
    }

    /// Parse a single variable definition line
    fn parse_variable_definition(
        &self,
        line: &str,
        line_num: usize,
        state: &mut CompilerState,
    ) -> Result<()> {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(CompilerError::variable(
                line_num,
                format!("Invalid variable definition syntax: '{}'. Expected 'name: value'", line)
            ));
        }

        let var_name = parts[0].trim();
        let raw_value = parts[1].trim();

        if !is_valid_identifier(var_name) {
            return Err(CompilerError::variable(
                line_num,
                format!("Invalid variable name '{}'", var_name)
            ));
        }

        // Warn about redefinition
        if let Some(existing) = state.variables.get(var_name) {
            log::warn!(
                "Line {}: Variable '{}' redefined. Previous definition at line {}",
                line_num, var_name, existing.def_line
            );
        }

        state.variables.insert(var_name.to_string(), VariableDef {
            value: String::new(),
            raw_value: raw_value.to_string(),
            def_line: line_num,
            is_resolving: false,
            is_resolved: false,
        });

        Ok(())
    }

    /// Resolve all variables, handling dependencies and detecting cycles
    fn resolve_all_variables(&self, state: &mut CompilerState) -> Result<()> {
        let var_names: Vec<String> = state.variables.keys().cloned().collect();
        
        for name in var_names {
            if !state.variables[&name].is_resolved {
                self.resolve_variable(&name, state, &mut HashSet::new())?;
            }
        }

        Ok(())
    }

    /// Recursively resolve a single variable
    fn resolve_variable(
        &self,
        name: &str,
        state: &mut CompilerState,
        visited: &mut HashSet<String>,
    ) -> Result<String> {
        // Check if variable exists
        let var_def = state.variables.get(name).ok_or_else(|| {
            CompilerError::variable(0, format!("Undefined variable '{}'", name))
        })?.clone();

        if var_def.is_resolved {
            return Ok(var_def.value);
        }

        if var_def.is_resolving || visited.contains(name) {
            let cycle_path: Vec<_> = visited.iter().cloned().collect();
            return Err(CompilerError::variable(
                var_def.def_line,
                format!("Circular variable dependency detected: {} -> {}", 
                       cycle_path.join(" -> "), name)
            ));
        }

        // Mark as resolving
        visited.insert(name.to_string());
        if let Some(var_def) = state.variables.get_mut(name) {
            var_def.is_resolving = true;
        }

        // Resolve dependencies in raw_value
        let mut current_value = var_def.raw_value.clone();
        
        // Find all variable references
        for captures in self.var_usage_regex.captures_iter(&var_def.raw_value) {
            let var_ref = captures.get(0).unwrap().as_str(); // $var_name
            let ref_var_name = captures.get(1).unwrap().as_str(); // var_name
            
            // Recursively resolve the referenced variable
            let resolved_ref_value = self.resolve_variable(ref_var_name, state, visited)?;
            
            // Replace in current value
            current_value = current_value.replace(var_ref, &resolved_ref_value);
        }

        // Update the variable with resolved value
        if let Some(var_def) = state.variables.get_mut(name) {
            var_def.value = current_value.clone();
            var_def.is_resolved = true;
            var_def.is_resolving = false;
        }

        visited.remove(name);
        Ok(current_value)
    }

    /// Remove @variables blocks and substitute $varName references
    fn perform_substitution_and_remove_blocks(
        &self,
        source: &str,
        state: &CompilerState,
    ) -> Result<String> {
        let mut result = String::new();
        let mut in_variables_block = false;
        let mut line_num = 0;
        let mut substitution_errors = Vec::new();

        for line in source.lines() {
            line_num += 1;
            let trimmed = line.trim();

            // Handle @variables block boundaries
            if trimmed.starts_with("@variables {") {
                in_variables_block = true;
                continue;
            }
            if in_variables_block && trimmed == "}" {
                in_variables_block = false;
                continue;
            }
            if in_variables_block {
                continue; // Skip lines inside @variables blocks
            }

            // Perform variable substitution on lines outside @variables blocks
            let substituted_line = self.var_usage_regex.replace_all(line, |caps: &regex::Captures| {
                let var_name = &caps[1];
                
                match state.variables.get(var_name) {
                    Some(var_def) if var_def.is_resolved => var_def.value.clone(),
                    Some(_) => {
                        substitution_errors.push(format!(
                            "Line {}: Internal error: variable '{}' used but not resolved",
                            line_num, var_name
                        ));
                        caps[0].to_string() // Return original if error
                    }
                    None => {
                        substitution_errors.push(format!(
                            "Line {}: Undefined variable '${}'",
                            line_num, var_name
                        ));
                        caps[0].to_string() // Return original if error
                    }
                }
            });

            result.push_str(&substituted_line);
            result.push('\n');
        }

        if !substitution_errors.is_empty() {
            return Err(CompilerError::variable(0, substitution_errors.join("\n")));
        }

        Ok(result)
    }

    /// Strip trailing comments from a line, respecting quotes
    fn strip_trailing_comment<'a>(&self, line: &'a str) -> &'a str {
        let mut in_quotes = false;
        let mut comment_pos = None;

        for (i, ch) in line.char_indices() {
            if ch == '"' {
                in_quotes = !in_quotes;
            }
            if ch == '#' && !in_quotes {
                comment_pos = Some(i);
                break;
            }
        }

        if let Some(pos) = comment_pos {
            line[..pos].trim_end()
        } else {
            line
        }
    }
}

// Update the existing evaluate_expression function in utils.rs:
pub fn evaluate_expression(expr: &str, variables: &HashMap<String, String>) -> Result<f64> {
    let mut expr = expr.to_string();
    
    // Replace variables with their values
    for (var_name, var_value) in variables {
        let var_ref = format!("${}", var_name);
        if expr.contains(&var_ref) {
            // Try to parse the variable value as a number
            let num_value = var_value.parse::<f64>().map_err(|_| {
                CompilerError::Variable {
                    line: 0,
                    message: format!("Variable ${} is not numeric: {}", var_name, var_value),
                }
            })?;
            expr = expr.replace(&var_ref, &num_value.to_string());
        }
    }
    
    // Use meval to evaluate the expression
    meval::eval_str(&expr).map_err(|e| CompilerError::Variable {
        line: 0,
        message: format!("Failed to evaluate expression '{}': {}", expr, e),
    })
}

/// Clean and parse a quoted or unquoted value string
pub fn clean_and_quote_value(value: &str) -> (String, bool) {
    let trimmed = value.trim();
    
    // Check for full-line comment
    if trimmed.starts_with('#') {
        return (String::new(), false);
    }
    
    // Find comment (not inside quotes)
    let mut in_quotes = false;
    let mut comment_index = None;
    
    for (i, ch) in trimmed.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
        }
        if ch == '#' && !in_quotes && i > 0 {
            comment_index = Some(i);
            break;
        }
    }
    
    let value_part = if let Some(idx) = comment_index {
        trimmed[..idx].trim_end()
    } else {
        trimmed
    };
    
    // Check if quoted
    let was_quoted = value_part.len() >= 2 
        && value_part.starts_with('"') 
        && value_part.ends_with('"');
    
    let cleaned = if was_quoted {
        value_part[1..value_part.len()-1].to_string()
    } else {
        value_part.to_string()
    };
    
    (cleaned, was_quoted)
}

/// Parse a color string (#RGB, #RRGGBB, #RRGGBBAA)
pub fn parse_color(color_str: &str) -> Result<Color> {
    let trimmed = color_str.trim();
    
    if !trimmed.starts_with('#') {
        return Err(CompilerError::InvalidFormat {
            message: format!("Color must start with #: {}", color_str),
        });
    }
    
    let hex_str = &trimmed[1..];
    
    match hex_str.len() {
        3 => {
            // RGB -> RRGGBB
            let r = u8::from_str_radix(&hex_str[0..1].repeat(2), 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            let g = u8::from_str_radix(&hex_str[1..2].repeat(2), 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            let b = u8::from_str_radix(&hex_str[2..3].repeat(2), 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            Ok(Color::new(r, g, b, 255))
        }
        4 => {
            // RGBA -> RRGGBBAA
            let r = u8::from_str_radix(&hex_str[0..1].repeat(2), 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            let g = u8::from_str_radix(&hex_str[1..2].repeat(2), 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            let b = u8::from_str_radix(&hex_str[2..3].repeat(2), 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            let a = u8::from_str_radix(&hex_str[3..4].repeat(2), 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            Ok(Color::new(r, g, b, a))
        }
        6 => {
            // RRGGBB
            let r = u8::from_str_radix(&hex_str[0..2], 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            let g = u8::from_str_radix(&hex_str[2..4], 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            let b = u8::from_str_radix(&hex_str[4..6], 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            Ok(Color::new(r, g, b, 255))
        }
        8 => {
            // RRGGBBAA
            let r = u8::from_str_radix(&hex_str[0..2], 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            let g = u8::from_str_radix(&hex_str[2..4], 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            let b = u8::from_str_radix(&hex_str[4..6], 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            let a = u8::from_str_radix(&hex_str[6..8], 16).map_err(|_| {
                CompilerError::InvalidFormat {
                    message: format!("Invalid hex color: {}", color_str),
                }
            })?;
            Ok(Color::new(r, g, b, a))
        }
        _ => Err(CompilerError::InvalidFormat {
            message: format!("Invalid color format: {} (expected 3, 4, 6, or 8 hex digits)", color_str),
        }),
    }
}

/// Parse layout string into layout byte
pub fn parse_layout_string(layout_str: &str) -> u8 {
    use crate::types::*;
    
    let parts: Vec<&str> = layout_str.split_whitespace().collect();
    let mut layout_byte = 0u8;
    
    let mut has_explicit_direction = false;
    let mut has_explicit_alignment = false;
    
    // Check for explicit settings
    for part in &parts {
        match *part {
            "row" | "col" | "column" | "row_rev" | "row-rev" | "col_rev" | "col-rev" | "column-rev" => {
                has_explicit_direction = true;
            }
            "start" | "center" | "centre" | "end" | "space_between" | "space-between" => {
                has_explicit_alignment = true;
            }
            _ => {}
        }
    }
    
    // Apply direction (default to Column)
    if !has_explicit_direction {
        layout_byte |= LAYOUT_DIRECTION_COLUMN;
    }
    
    for part in &parts {
        match *part {
            "row" => {
                layout_byte = (layout_byte & !LAYOUT_DIRECTION_MASK) | LAYOUT_DIRECTION_ROW;
            }
            "col" | "column" => {
                layout_byte = (layout_byte & !LAYOUT_DIRECTION_MASK) | LAYOUT_DIRECTION_COLUMN;
            }
            "row_rev" | "row-rev" => {
                layout_byte = (layout_byte & !LAYOUT_DIRECTION_MASK) | LAYOUT_DIRECTION_ROW_REV;
            }
            "col_rev" | "col-rev" | "column-rev" => {
                layout_byte = (layout_byte & !LAYOUT_DIRECTION_MASK) | LAYOUT_DIRECTION_COL_REV;
            }
            _ => {}
        }
    }
    
    // Apply alignment (default to Start)
    if !has_explicit_alignment {
        layout_byte |= LAYOUT_ALIGNMENT_START;
    }
    
    for part in &parts {
        match *part {
            "start" => {
                layout_byte = (layout_byte & !LAYOUT_ALIGNMENT_MASK) | LAYOUT_ALIGNMENT_START;
            }
            "center" | "centre" => {
                layout_byte = (layout_byte & !LAYOUT_ALIGNMENT_MASK) | LAYOUT_ALIGNMENT_CENTER;
            }
            "end" => {
                layout_byte = (layout_byte & !LAYOUT_ALIGNMENT_MASK) | LAYOUT_ALIGNMENT_END;
            }
            "space_between" | "space-between" => {
                layout_byte = (layout_byte & !LAYOUT_ALIGNMENT_MASK) | LAYOUT_ALIGNMENT_SPACE_BETWEEN;
            }
            _ => {}
        }
    }
    
    // Apply flags
    for part in &parts {
        match *part {
            "wrap" => layout_byte |= LAYOUT_WRAP_BIT,
            "grow" => layout_byte |= LAYOUT_GROW_BIT,
            "absolute" => layout_byte |= LAYOUT_ABSOLUTE_BIT,
            _ => {}
        }
    }
    
    layout_byte
}

/// Guess resource type from property key
pub fn guess_resource_type(key: &str) -> crate::types::ResourceType {
    use crate::types::ResourceType;
    
    let lower_key = key.to_lowercase();
    
    if lower_key.contains("image") || lower_key.contains("icon") || 
       lower_key.contains("sprite") || lower_key.contains("texture") ||
       lower_key.contains("background") || lower_key.contains("logo") ||
       lower_key.contains("avatar") {
        ResourceType::Image
    } else if lower_key.contains("font") {
        ResourceType::Font
    } else if lower_key.contains("sound") || lower_key.contains("audio") ||
              lower_key.contains("music") {
        ResourceType::Sound
    } else if lower_key.contains("video") {
        ResourceType::Video
    } else {
        ResourceType::Image // Default
    }
}

/// Convert a 64-bit float to 8.8 fixed point (16-bit)
pub fn float_to_fixed_point(value: f64) -> u16 {
    (value * 256.0).round() as u16
}

/// Convert 8.8 fixed point to 64-bit float
pub fn fixed_point_to_float(value: u16) -> f64 {
    value as f64 / 256.0
}

/// Simple arithmetic expression evaluator
fn eval_arithmetic(expr: &str) -> std::result::Result<f64, String> {
    let expr = expr.trim();
    
    // Try direct parsing first
    if let Ok(num) = expr.parse::<f64>() {
        return Ok(num);
    }
    
    // Handle simple binary operations
    for op in &["*", "/", "+", "-"] {
        if let Some(pos) = expr.rfind(op) {
            // Skip if it's at the beginning (unary minus)
            if pos == 0 && *op == "-" {
                continue;
            }
            
            let left = expr[..pos].trim();
            let right = expr[pos + 1..].trim();
            
            let left_val = eval_arithmetic(left)?;
            let right_val = eval_arithmetic(right)?;
            
            return match *op {
                "*" => Ok(left_val * right_val),
                "/" => {
                    if right_val == 0.0 {
                        Err("Division by zero".to_string())
                    } else {
                        Ok(left_val / right_val)
                    }
                }
                "+" => Ok(left_val + right_val),
                "-" => Ok(left_val - right_val),
                _ => unreachable!(),
            };
        }
    }
    
    // Handle parentheses
    if expr.starts_with('(') && expr.ends_with(')') {
        return eval_arithmetic(&expr[1..expr.len() - 1]);
    }
    
    Err(format!("Cannot evaluate expression: {}", expr))
}

/// Check if a string is a valid identifier
pub fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Split properties string by semicolon, respecting quotes
pub fn split_properties_by_semicolon(props_str: &str) -> Vec<String> {
    let mut properties = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;
    
    for ch in props_str.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }
        
        if ch == '\\' {
            current.push(ch);
            escape_next = true;
            continue;
        }
        
        if ch == '"' {
            in_quotes = !in_quotes;
            current.push(ch);
            continue;
        }
        
        if ch == ';' && !in_quotes {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                properties.push(trimmed.to_string());
            }
            current.clear();
            continue;
        }
        
        current.push(ch);
    }
    
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        properties.push(trimmed.to_string());
    }
    
    properties
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_clean_and_quote_value() {
        let (cleaned, quoted) = clean_and_quote_value(r#""hello world""#);
        assert_eq!(cleaned, "hello world");
        assert!(quoted);
        
        let (cleaned, quoted) = clean_and_quote_value("unquoted");
        assert_eq!(cleaned, "unquoted");
        assert!(!quoted);
        
        let (cleaned, _) = clean_and_quote_value("value # comment");
        assert_eq!(cleaned, "value");
    }
    
    #[test]
    fn test_parse_color() {
        let color = parse_color("#FF0000").unwrap();
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 0);
        assert_eq!(color.b, 0);
        assert_eq!(color.a, 255);
        
        let color = parse_color("#RGB").unwrap();
        assert_eq!(color.r, 204); // R -> RR
        assert_eq!(color.g, 204); // G -> GG
        assert_eq!(color.b, 68);  // B -> BB
        
        let color = parse_color("#12345678").unwrap();
        assert_eq!(color.r, 0x12);
        assert_eq!(color.g, 0x34);
        assert_eq!(color.b, 0x56);
        assert_eq!(color.a, 0x78);
    }
    
    #[test]
    fn test_parse_layout_string() {
        assert_eq!(parse_layout_string("row center"), 
                   LAYOUT_DIRECTION_ROW | LAYOUT_ALIGNMENT_CENTER);
        assert_eq!(parse_layout_string("column start grow"), 
                   LAYOUT_DIRECTION_COLUMN | LAYOUT_ALIGNMENT_START | LAYOUT_GROW_BIT);
    }
    
    #[test]
    fn test_evaluate_expression() {
        let mut vars = HashMap::new();
        vars.insert("base".to_string(), "10".to_string());
        vars.insert("factor".to_string(), "2".to_string());
        
        assert_eq!(evaluate_expression("$base * $factor", &vars).unwrap(), 20.0);
        assert_eq!(evaluate_expression("$base + 5", &vars).unwrap(), 15.0);
        assert_eq!(evaluate_expression("42", &vars).unwrap(), 42.0);
    }
    
    #[test]
    fn test_split_properties_by_semicolon() {
        let props = r#"key1: "value with ; semicolon"; key2: value2; key3: "another value""#;
        let split = split_properties_by_semicolon(props);
        
        assert_eq!(split.len(), 3);
        assert_eq!(split[0], r#"key1: "value with ; semicolon""#);
        assert_eq!(split[1], "key2: value2");
        assert_eq!(split[2], r#"key3: "another value""#);
    }
}