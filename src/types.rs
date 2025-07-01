//! Core types and constants for the Kryon compiler

use std::collections::HashMap;
use std::fmt;

// KRB Format Constants
pub const KRB_MAGIC: &[u8; 4] = b"KRB1";
pub const KRB_VERSION_MAJOR: u8 = 0;
pub const KRB_VERSION_MINOR: u8 = 5;
pub const KRB_HEADER_SIZE: usize = 54;
pub const KRB_ELEMENT_HEADER_SIZE: usize = 18;

// Header Flags
pub const FLAG_HAS_STYLES: u16 = 1 << 0;
pub const FLAG_HAS_COMPONENT_DEFS: u16 = 1 << 1;
pub const FLAG_HAS_ANIMATIONS: u16 = 1 << 2;
pub const FLAG_HAS_RESOURCES: u16 = 1 << 3;
pub const FLAG_COMPRESSED: u16 = 1 << 4;
pub const FLAG_FIXED_POINT: u16 = 1 << 5;
pub const FLAG_EXTENDED_COLOR: u16 = 1 << 6;
pub const FLAG_HAS_APP: u16 = 1 << 7;
pub const FLAG_HAS_SCRIPTS: u16 = 1 << 8;
pub const FLAG_HAS_STATE_PROPERTIES: u16 = 1 << 9;

// Element Types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ElementType {
    App = 0x00,
    Container = 0x01,
    Text = 0x02,
    Image = 0x03,
    Canvas = 0x04,
    Button = 0x10,
    Input = 0x11,
    Checkbox = 0x12,
    Radio = 0x13,
    Slider = 0x14,
    List = 0x20,
    Grid = 0x21,
    Scrollable = 0x22,
    Tabs = 0x23,
    Video = 0x30,
    InternalComponentUsage = 0xFE,
    Unknown = 0xFF,
    CustomBase = 0x31,
}

impl ElementType {
    pub fn from_name(name: &str) -> Self {
        match name {
            "App" => Self::App,
            "Container" => Self::Container,
            "Text" => Self::Text,
            "Image" => Self::Image,
            "Canvas" => Self::Canvas,
            "Button" => Self::Button,
            "Input" => Self::Input,
            "Checkbox" => Self::Checkbox,
            "Radio" => Self::Radio,
            "Slider" => Self::Slider,
            "List" => Self::List,
            "Grid" => Self::Grid,
            "Scrollable" => Self::Scrollable,
            "Tabs" => Self::Tabs,
            "Video" => Self::Video,
            _ => Self::Unknown,
        }
    }
}

impl PropertyId {
    /// Centralized property name to ID mapping - used by all compilation phases
    pub fn from_name(key: &str) -> Self {
        match key {
            "background_color" => PropertyId::BackgroundColor,     // 0x01
            "text_color" | "foreground_color" => PropertyId::ForegroundColor,  // 0x02  
            "border_color" => PropertyId::BorderColor,             // 0x03
            "border_width" => PropertyId::BorderWidth,             // 0x04
            "border_radius" => PropertyId::BorderRadius,           // 0x05
            "padding" => PropertyId::Padding,                      // 0x06
            "margin" => PropertyId::Margin,                        // 0x07
            "text" => PropertyId::TextContent,                     // 0x08
            "font_size" => PropertyId::FontSize,                   // 0x09
            "font_weight" => PropertyId::FontWeight,               // 0x0A
            "text_alignment" => PropertyId::TextAlignment,         // 0x0B
            "src" => PropertyId::ImageSource,                      // 0x0C
            "opacity" => PropertyId::Opacity,                      // 0x0D
            "z_index" => PropertyId::ZIndex,                       // 0x0E
            "visibility" => PropertyId::Visibility,                // 0x0F
            "gap" => PropertyId::Gap,                              // 0x10
            "min_width" => PropertyId::MinWidth,                   // 0x11
            "min_height" => PropertyId::MinHeight,                 // 0x12
            "max_width" => PropertyId::MaxWidth,                   // 0x13
            "max_height" => PropertyId::MaxHeight,                 // 0x14
            "aspect_ratio" => PropertyId::AspectRatio,             // 0x15
            "transform" => PropertyId::Transform,                  // 0x16
            "shadow" => PropertyId::Shadow,                        // 0x17
            "overflow" => PropertyId::Overflow,                    // 0x18
            "cursor" => PropertyId::Cursor,                        // 0x29
            
            // App-specific properties (0x20-0x28)
            "window_width" => PropertyId::WindowWidth,             // 0x20
            "window_height" => PropertyId::WindowHeight,           // 0x21
            "window_title" => PropertyId::WindowTitle,             // 0x22
            "resizable" => PropertyId::Resizable,                  // 0x23
            "keep_aspect_ratio" => PropertyId::KeepAspect,         // 0x24
            "scale_factor" => PropertyId::ScaleFactor,             // 0x25
            "icon" => PropertyId::Icon,                            // 0x26
            "version" => PropertyId::Version,                      // 0x27
            "author" => PropertyId::Author,                        // 0x28
            
            _ => PropertyId::Invalid,
        }
    }
    
    /// Check if this property should be handled in the element header instead of as a KRB property
    pub fn is_element_header_property(key: &str) -> bool {
        matches!(key, "pos_x" | "pos_y" | "width" | "height" | "style" | "layout")
    }
}

// Property IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PropertyId {
    Invalid = 0x00,
    BackgroundColor = 0x01,
    ForegroundColor = 0x02,
    BorderColor = 0x03,
    BorderWidth = 0x04,
    BorderRadius = 0x05,
    Padding = 0x06,
    Margin = 0x07,
    TextContent = 0x08,
    FontSize = 0x09,
    FontWeight = 0x0A,
    TextAlignment = 0x0B,
    ImageSource = 0x0C,
    Opacity = 0x0D,
    ZIndex = 0x0E,
    Visibility = 0x0F,
    Gap = 0x10,
    MinWidth = 0x11,
    MinHeight = 0x12,
    MaxWidth = 0x13,
    MaxHeight = 0x14,
    AspectRatio = 0x15,
    Transform = 0x16,
    Shadow = 0x17,
    Overflow = 0x18,
    CustomData = 0x19,
    LayoutFlags = 0x1A,
    // App-specific properties
    WindowWidth = 0x20,
    WindowHeight = 0x21,
    WindowTitle = 0x22,
    Resizable = 0x23,
    KeepAspect = 0x24,
    ScaleFactor = 0x25,
    Icon = 0x26,
    Version = 0x27,
    Author = 0x28,
    Cursor = 0x29,
}

// Value Types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ValueType {
    None = 0x00,
    Byte = 0x01,
    Short = 0x02,
    Color = 0x03,
    String = 0x04,
    Resource = 0x05,
    Percentage = 0x06,
    Rect = 0x07,
    EdgeInsets = 0x08,
    Enum = 0x09,
    Vector = 0x0A,
    Custom = 0x0B,
    // Hint types (for parsing)
    StyleId = 0x0C,
    Float = 0x0D,
    Int = 0x0E,
    Bool = 0x0F,
}

// Layout flags
pub const LAYOUT_DIRECTION_MASK: u8 = 0x03;
pub const LAYOUT_DIRECTION_ROW: u8 = 0;
pub const LAYOUT_DIRECTION_COLUMN: u8 = 1;
pub const LAYOUT_DIRECTION_ROW_REV: u8 = 2;
pub const LAYOUT_DIRECTION_COL_REV: u8 = 3;

pub const LAYOUT_ALIGNMENT_MASK: u8 = 0x0C;
pub const LAYOUT_ALIGNMENT_START: u8 = 0 << 2;
pub const LAYOUT_ALIGNMENT_CENTER: u8 = 1 << 2;
pub const LAYOUT_ALIGNMENT_END: u8 = 2 << 2;
pub const LAYOUT_ALIGNMENT_SPACE_BETWEEN: u8 = 3 << 2;

pub const LAYOUT_WRAP_BIT: u8 = 1 << 4;
pub const LAYOUT_GROW_BIT: u8 = 1 << 5;
pub const LAYOUT_ABSOLUTE_BIT: u8 = 1 << 6;

// Script language IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ScriptLanguage {
    Lua = 0x01,
    JavaScript = 0x02,
    Python = 0x03,
    Wren = 0x04,
}

impl ScriptLanguage {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "lua" => Some(Self::Lua),
            "javascript" | "js" => Some(Self::JavaScript),
            "python" | "py" => Some(Self::Python),
            "wren" => Some(Self::Wren),
            _ => None,
        }
    }
}

// State flags for pseudo-selectors
pub const STATE_HOVER: u8 = 1 << 0;
pub const STATE_ACTIVE: u8 = 1 << 1;
pub const STATE_FOCUS: u8 = 1 << 2;
pub const STATE_DISABLED: u8 = 1 << 3;
pub const STATE_CHECKED: u8 = 1 << 4;

// Resource types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceType {
    Image = 0x01,
    Font = 0x02,
    Sound = 0x03,
    Video = 0x04,
    Script = 0x05,
    Custom = 0x06,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceFormat {
    External = 0x00,
    Inline = 0x01,
}

// Compiler limits
pub const MAX_ELEMENTS: usize = 1024;
pub const MAX_STRINGS: usize = 1024;
pub const MAX_PROPERTIES: usize = 64;
pub const MAX_CUSTOM_PROPERTIES: usize = 32;
pub const MAX_STYLES: usize = 256;
pub const MAX_CHILDREN: usize = 256;
pub const MAX_EVENTS: usize = 16;
pub const MAX_RESOURCES: usize = 256;
pub const MAX_INCLUDE_DEPTH: usize = 16;
pub const MAX_COMPONENT_DEFS: usize = 128;
pub const MAX_BLOCK_DEPTH: usize = 64;
pub const MAX_LINE_LENGTH: usize = 2048;

// Core data structures
#[derive(Debug, Clone)]
pub struct KrbProperty {
    pub property_id: u8,
    pub value_type: ValueType,
    pub size: u8,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct KrbCustomProperty {
    pub key_index: u8,
    pub value_type: ValueType,
    pub size: u8,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct StatePropertySet {
    pub state_flags: u8,
    pub property_count: u8,
    pub properties: Vec<KrbProperty>,
}

#[derive(Debug, Clone)]
pub struct SourceProperty {
    pub key: String,
    pub value: String,
    pub line_num: usize,
}

#[derive(Debug, Clone)]
pub struct KrbEvent {
    pub event_type: u8,
    pub callback_id: u8,
}

#[derive(Debug, Clone)]
pub struct StringEntry {
    pub text: String,
    pub length: usize,
    pub index: u8,
}

#[derive(Debug, Clone)]
pub struct ResourceEntry {
    pub resource_type: ResourceType,
    pub name_index: u8,
    pub format: ResourceFormat,
    pub data_string_index: u8,
    pub index: u8,
    pub calculated_size: u32,
}

#[derive(Debug, Clone)]
pub struct ComponentPropertyDef {
    pub name: String,
    pub value_type_hint: ValueType,
    pub default_value: String,
}

#[derive(Debug, Clone)]
pub struct ComponentDefinition {
    pub name: String,
    pub properties: Vec<ComponentPropertyDef>,
    pub definition_start_line: usize,
    pub definition_root_element_index: Option<usize>,
    pub calculated_size: u32,
    pub internal_template_element_offsets: HashMap<usize, u32>,
}

#[derive(Debug, Clone)]
pub struct ScriptFunction {
    pub function_name: String,
    pub function_name_index: u8,
}

#[derive(Debug, Clone)]
pub struct ScriptEntry {
    pub language_id: ScriptLanguage,
    pub name: String,
    pub name_index: u8,
    pub storage_format: u8,
    pub entry_point_count: u8,
    pub data_size: u16,
    pub entry_points: Vec<ScriptFunction>,
    pub code_data: Vec<u8>,
    pub resource_index: Option<u8>,
    pub calculated_size: u32,
    pub source_line_num: usize,
}

#[derive(Debug, Clone)]
pub struct StyleEntry {
    pub id: u8,
    pub source_name: String,
    pub name_index: u8,
    pub extends_style_names: Vec<String>,
    pub properties: Vec<KrbProperty>,
    pub source_properties: Vec<SourceProperty>,
    pub calculated_size: u32,
    pub is_resolved: bool,
    pub is_resolving: bool,
}

#[derive(Debug, Clone)]
pub struct Element {
    // KRB header fields
    pub element_type: ElementType,
    pub id_string_index: u8,
    pub pos_x: u16,
    pub pos_y: u16,
    pub width: u16,
    pub height: u16,
    pub layout: u8,
    pub style_id: u8,
    pub property_count: u8,
    pub child_count: u8,
    pub event_count: u8,
    pub animation_count: u8,
    pub custom_prop_count: u8,
    pub state_prop_count: u8,

    // Resolved KRB data
    pub krb_properties: Vec<KrbProperty>,
    pub krb_custom_properties: Vec<KrbCustomProperty>,
    pub krb_events: Vec<KrbEvent>,
    pub state_property_sets: Vec<StatePropertySet>,
    pub children: Vec<usize>, // Indices into the elements array

    // Compiler state
    pub parent_index: Option<usize>,
    pub self_index: usize,
    pub is_component_instance: bool,
    pub component_def: Option<usize>, // Index into component defs
    pub is_definition_root: bool,
    pub source_element_name: String,
    pub source_id_name: String,
    pub source_properties: Vec<SourceProperty>,
    pub source_children_indices: Vec<usize>,
    pub source_line_num: usize,
    pub layout_flags_source: u8,
    pub position_hint: String,
    pub orientation_hint: String,

    // Writing data
    pub calculated_size: u32,
    pub absolute_offset: u32,
    pub processed_in_pass: bool,
}

#[derive(Debug, Clone)]
pub struct VariableDef {
    pub value: String,
    pub raw_value: String,
    pub def_line: usize,
    pub is_resolving: bool,
    pub is_resolved: bool,
}

#[derive(Debug)]
pub struct CompilerState {
    pub elements: Vec<Element>,
    pub strings: Vec<StringEntry>,
    pub styles: Vec<StyleEntry>,
    pub scripts: Vec<ScriptEntry>,
    pub resources: Vec<ResourceEntry>,
    pub component_defs: Vec<ComponentDefinition>,
    pub variables: HashMap<String, VariableDef>,

    pub has_app: bool,
    pub header_flags: u16,

    // Parser state
    pub current_line_num: usize,
    pub current_file_path: String,

    // Calculated offsets
    pub element_offset: u32,
    pub style_offset: u32,
    pub component_def_offset: u32,
    pub anim_offset: u32,
    pub script_offset: u32,
    pub string_offset: u32,
    pub resource_offset: u32,
    pub total_size: u32,

    // Section sizes
    pub total_element_data_size: u32,
    pub total_style_data_size: u32,
    pub total_component_def_data_size: u32,
    pub total_script_data_size: u32,
    pub total_string_data_size: u32,
    pub total_resource_table_size: u32,
}

impl CompilerState {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            strings: Vec::new(),
            styles: Vec::new(),
            scripts: Vec::new(),
            resources: Vec::new(),
            component_defs: Vec::new(),
            variables: HashMap::new(),
            has_app: false,
            header_flags: 0,
            current_line_num: 0,
            current_file_path: String::new(),
            element_offset: 0,
            style_offset: 0,
            component_def_offset: 0,
            anim_offset: 0,
            script_offset: 0,
            string_offset: 0,
            resource_offset: 0,
            total_size: 0,
            total_element_data_size: 0,
            total_style_data_size: 0,
            total_component_def_data_size: 0,
            total_script_data_size: 0,
            total_string_data_size: 0,
            total_resource_table_size: 0,
        }
    }
}

// Color utilities
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }
}