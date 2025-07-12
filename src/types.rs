//! Core types and constants for the Kryon compiler

use std::collections::HashMap;
use std::fmt;
use crate::ast::AstNode;

// KRB Format Constants
pub const KRB_MAGIC: &[u8; 4] = b"KRB1";
pub const KRB_VERSION_MAJOR: u8 = 0;
pub const KRB_VERSION_MINOR: u8 = 5;
pub const KRB_HEADER_SIZE: usize = 72;
pub const KRB_ELEMENT_HEADER_SIZE: usize = 19;

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
pub const FLAG_HAS_TEMPLATE_VARIABLES: u16 = 1 << 10;
pub const FLAG_HAS_TRANSFORMS: u16 = 1 << 11;

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
            "List" => Self::List,
            "Grid" => Self::Grid,
            "Scrollable" => Self::Scrollable,
            "Tabs" => Self::Tabs,
            "Video" => Self::Video,
            _ => Self::Unknown,
        }
    }
}

// Input Types for the unified Input element
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InputType {
    // Textual inputs
    Text = 0x00,           // Default type
    Password = 0x01,
    Email = 0x02,
    Number = 0x03,
    Tel = 0x04,
    Url = 0x05,
    Search = 0x06,
    
    // Selection inputs
    Checkbox = 0x10,
    Radio = 0x11,
    
    // Range input
    Range = 0x20,
    
    // Date and time inputs
    Date = 0x30,
    DatetimeLocal = 0x31,
    Month = 0x32,
    Time = 0x33,
    Week = 0x34,
    
    // Specialized inputs
    Color = 0x40,
    File = 0x41,
    Hidden = 0x42,
    
    // Button inputs
    Submit = 0x50,
    Reset = 0x51,
    Button = 0x52,
    Image = 0x53,
}

impl InputType {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "text" => Some(Self::Text),
            "password" => Some(Self::Password),
            "email" => Some(Self::Email),
            "number" => Some(Self::Number),
            "tel" => Some(Self::Tel),
            "url" => Some(Self::Url),
            "search" => Some(Self::Search),
            "checkbox" => Some(Self::Checkbox),
            "radio" => Some(Self::Radio),
            "range" => Some(Self::Range),
            "date" => Some(Self::Date),
            "datetime-local" => Some(Self::DatetimeLocal),
            "month" => Some(Self::Month),
            "time" => Some(Self::Time),
            "week" => Some(Self::Week),
            "color" => Some(Self::Color),
            "file" => Some(Self::File),
            "hidden" => Some(Self::Hidden),
            "submit" => Some(Self::Submit),
            "reset" => Some(Self::Reset),
            "button" => Some(Self::Button),
            "image" => Some(Self::Image),
            _ => None,
        }
    }
    
    pub fn to_name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Password => "password",
            Self::Email => "email",
            Self::Number => "number",
            Self::Tel => "tel",
            Self::Url => "url",
            Self::Search => "search",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Range => "range",
            Self::Date => "date",
            Self::DatetimeLocal => "datetime-local",
            Self::Month => "month",
            Self::Time => "time",
            Self::Week => "week",
            Self::Color => "color",
            Self::File => "file",
            Self::Hidden => "hidden",
            Self::Submit => "submit",
            Self::Reset => "reset",
            Self::Button => "button",
            Self::Image => "image",
        }
    }
    
    /// Returns true if this input type supports textual input
    pub fn is_textual(self) -> bool {
        matches!(self, Self::Text | Self::Password | Self::Email | 
                      Self::Number | Self::Tel | Self::Url | Self::Search)
    }
    
    /// Returns true if this input type is a selection control
    pub fn is_selection(self) -> bool {
        matches!(self, Self::Checkbox | Self::Radio)
    }
    
    /// Returns true if this input type supports min/max/step properties
    pub fn supports_range(self) -> bool {
        matches!(self, Self::Number | Self::Range | Self::Date | 
                      Self::DatetimeLocal | Self::Month | Self::Time | Self::Week)
    }
    
    /// Returns true if this input type supports the checked property
    pub fn supports_checked(self) -> bool {
        matches!(self, Self::Checkbox | Self::Radio)
    }
}

impl Default for InputType {
    fn default() -> Self {
        Self::Text
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
            "font_family" => PropertyId::FontFamily,               // 0x0C
            "src" | "image_source" => PropertyId::ImageSource,     // 0x0D
            "opacity" => PropertyId::Opacity,                      // 0x0E
            "z_index" => PropertyId::ZIndex,                       // 0x0F
            "visibility" | "visible" => PropertyId::Visibility,    // 0x10
            "gap" => PropertyId::Gap,                              // 0x11
            "min_width" => PropertyId::MinWidth,                   // 0x12
            "min_height" => PropertyId::MinHeight,                 // 0x13
            "max_width" => PropertyId::MaxWidth,                   // 0x14
            "max_height" => PropertyId::MaxHeight,                 // 0x15
            "aspect_ratio" => PropertyId::AspectRatio,             // 0x16
            "transform" => PropertyId::Transform,                  // 0x17
            "shadow" => PropertyId::Shadow,                        // 0x18
            "overflow" => PropertyId::Overflow,                    // 0x19
            "width" => PropertyId::Width,                          // 0x1A
            "height" => PropertyId::Height,                        // 0x1C
            "cursor" => PropertyId::Cursor,                        // 0x29
            "checked" => PropertyId::Checked,                      // 0x2A
            "type" => PropertyId::InputType,                       // 0x2B
            
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
            
            // CSS Grid Properties (0x60-0x6F - matching renderer PropertyRegistry)
            "grid_template_columns" | "grid-template-columns" => PropertyId::GridTemplateColumns, // 0x60
            "grid_template_rows" | "grid-template-rows" => PropertyId::GridTemplateRows,           // 0x61
            "grid_template_areas" | "grid-template-areas" => PropertyId::GridTemplateAreas,       // 0x62
            "grid_auto_columns" | "grid-auto-columns" => PropertyId::GridAutoColumns,             // 0x63
            "grid_auto_rows" | "grid-auto-rows" => PropertyId::GridAutoRows,                      // 0x64
            "grid_auto_flow" | "grid-auto-flow" => PropertyId::GridAutoFlow,                      // 0x65
            "grid_area" | "grid-area" => PropertyId::GridArea,                                     // 0x66
            "grid_column" | "grid-column" => PropertyId::GridColumn,                               // 0x67
            "grid_row" | "grid-row" => PropertyId::GridRow,                                        // 0x68
            "grid_column_start" | "grid-column-start" => PropertyId::GridColumnStart,             // 0x69
            "grid_column_end" | "grid-column-end" => PropertyId::GridColumnEnd,                   // 0x6A
            "grid_row_start" | "grid-row-start" => PropertyId::GridRowStart,                      // 0x6B
            "grid_row_end" | "grid-row-end" => PropertyId::GridRowEnd,                            // 0x6C
            "grid_gap" | "grid-gap" => PropertyId::GridGap,                                        // 0x6D
            "grid_column_gap" | "grid-column-gap" | "column_gap" => PropertyId::GridColumnGap,    // 0x6E
            "grid_row_gap" | "grid-row-gap" | "row_gap" => PropertyId::GridRowGap,                // 0x6F
            
            // Modern Flexbox Properties (0x40-0x4F)
            "display" => PropertyId::Display,                                                     // 0x40
            "flex_direction" | "flex-direction" => PropertyId::FlexDirection,                     // 0x41
            "flex_wrap" | "flex-wrap" => PropertyId::FlexWrap,                                     // 0x42
            "flex_grow" | "flex-grow" => PropertyId::FlexGrow,                                     // 0x43
            "flex_shrink" | "flex-shrink" => PropertyId::FlexShrink,                              // 0x44
            "flex_basis" | "flex-basis" => PropertyId::FlexBasis,                                 // 0x45
            "align_items" | "align-items" => PropertyId::AlignItems,                              // 0x46
            "align_self" | "align-self" => PropertyId::AlignSelf,                                 // 0x47
            "align_content" | "align-content" => PropertyId::AlignContent,                        // 0x48
            "justify_content" | "justify-content" => PropertyId::JustifyContent,                  // 0x49
            "justify_items" | "justify-items" => PropertyId::JustifyItems,                        // 0x4A
            "justify_self" | "justify-self" => PropertyId::JustifySelf,                           // 0x4B
            "order" => PropertyId::Order,                                                          // 0x4C
            
            // Positioning Properties (0x50-0x5F)
            "position" => PropertyId::Position,                                                    // 0x50
            "top" => PropertyId::Top,                                                              // 0x51
            "right" => PropertyId::Right,                                                          // 0x52
            "bottom" => PropertyId::Bottom,                                                        // 0x53
            "left" => PropertyId::Left,                                                            // 0x54
            "inset" => PropertyId::Inset,                                                          // 0x55
            
            // Box Model Properties (0x70-0x8F)
            "padding_top" | "padding-top" => PropertyId::PaddingTop,                             // 0x71
            "padding_right" | "padding-right" => PropertyId::PaddingRight,                       // 0x72
            "padding_bottom" | "padding-bottom" => PropertyId::PaddingBottom,                    // 0x73
            "padding_left" | "padding-left" => PropertyId::PaddingLeft,                          // 0x74
            "margin_top" | "margin-top" => PropertyId::MarginTop,                                // 0x76
            "margin_right" | "margin-right" => PropertyId::MarginRight,                          // 0x77
            "margin_bottom" | "margin-bottom" => PropertyId::MarginBottom,                       // 0x78
            "margin_left" | "margin-left" => PropertyId::MarginLeft,                             // 0x79
            "border_top_width" | "border-top-width" => PropertyId::BorderTopWidth,               // 0x7A
            "border_right_width" | "border-right-width" => PropertyId::BorderRightWidth,         // 0x7B
            "border_bottom_width" | "border-bottom-width" => PropertyId::BorderBottomWidth,      // 0x7C
            "border_left_width" | "border-left-width" => PropertyId::BorderLeftWidth,            // 0x7D
            "border_top_color" | "border-top-color" => PropertyId::BorderTopColor,               // 0x7E
            "border_right_color" | "border-right-color" => PropertyId::BorderRightColor,         // 0x7F
            "border_bottom_color" | "border-bottom-color" => PropertyId::BorderBottomColor,      // 0x80
            "border_left_color" | "border-left-color" => PropertyId::BorderLeftColor,            // 0x81
            "border_top_left_radius" | "border-top-left-radius" => PropertyId::BorderTopLeftRadius,        // 0x82
            "border_top_right_radius" | "border-top-right-radius" => PropertyId::BorderTopRightRadius,     // 0x83
            "border_bottom_right_radius" | "border-bottom-right-radius" => PropertyId::BorderBottomRightRadius, // 0x84
            "border_bottom_left_radius" | "border-bottom-left-radius" => PropertyId::BorderBottomLeftRadius,     // 0x85
            "box_sizing" | "box-sizing" => PropertyId::BoxSizing,                                // 0x86
            "outline" => PropertyId::Outline,                                                     // 0x87
            "outline_color" | "outline-color" => PropertyId::OutlineColor,                       // 0x88
            "outline_width" | "outline-width" => PropertyId::OutlineWidth,                       // 0x89
            "outline_offset" | "outline-offset" => PropertyId::OutlineOffset,                    // 0x8A
            
            // Sizing Properties (0x90-0x9F)
            "min_size" | "min-size" => PropertyId::MinSize,                                       // 0x90
            "max_size" | "max-size" => PropertyId::MaxSize,                                       // 0x91
            "preferred_size" | "preferred-size" => PropertyId::PreferredSize,                     // 0x92
            
            _ => PropertyId::CustomData,
        }
    }
    
    /// Check if this property should be handled ONLY in the element header and never as a style property
    /// These are truly element-specific properties that don't make sense as inheritable styles
    pub fn is_element_header_property(key: &str) -> bool {
        matches!(key, "id" | "checked")
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
    FontFamily = 0x0C,
    ImageSource = 0x0D,
    Opacity = 0x0E,
    ZIndex = 0x0F,
    Visibility = 0x10,
    Gap = 0x11,
    MinWidth = 0x12,
    MinHeight = 0x13,
    MaxWidth = 0x14,
    MaxHeight = 0x15,
    AspectRatio = 0x16,
    Transform = 0x17,
    Shadow = 0x18,
    Overflow = 0x19,
    Width = 0x1A,
    Height = 0x1C,
    CustomData = 0x1D,
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
    Checked = 0x2A,
    InputType = 0x2B,
    
    // CSS Grid Properties (0x60-0x6F - matching renderer PropertyRegistry)
    GridTemplateColumns = 0x60,
    GridTemplateRows = 0x61,
    GridTemplateAreas = 0x62,
    GridAutoColumns = 0x63,
    GridAutoRows = 0x64,
    GridAutoFlow = 0x65,
    GridArea = 0x66,
    GridColumn = 0x67,
    GridRow = 0x68,
    GridColumnStart = 0x69,
    GridColumnEnd = 0x6A,
    GridRowStart = 0x6B,
    GridRowEnd = 0x6C,
    GridGap = 0x6D,
    GridColumnGap = 0x6E,
    GridRowGap = 0x6F,
    
    // Box Model Properties (0x70-0x8F)
    PaddingTop = 0x71,
    PaddingRight = 0x72,
    PaddingBottom = 0x73,
    PaddingLeft = 0x74,
    MarginTop = 0x76,
    MarginRight = 0x77,
    MarginBottom = 0x78,
    MarginLeft = 0x79,
    BorderTopWidth = 0x7A,
    BorderRightWidth = 0x7B,
    BorderBottomWidth = 0x7C,
    BorderLeftWidth = 0x7D,
    BorderTopColor = 0x7E,
    BorderRightColor = 0x7F,
    BorderBottomColor = 0x80,
    BorderLeftColor = 0x81,
    BorderTopLeftRadius = 0x82,
    BorderTopRightRadius = 0x83,
    BorderBottomRightRadius = 0x84,
    BorderBottomLeftRadius = 0x85,
    BoxSizing = 0x86,
    Outline = 0x87,
    OutlineColor = 0x88,
    OutlineWidth = 0x89,
    OutlineOffset = 0x8A,
    
    // Taffy Modern Flexbox Properties (0x40-0x4F)
    Display = 0x40,
    FlexDirection = 0x41,
    FlexWrap = 0x42,
    FlexGrow = 0x43,
    FlexShrink = 0x44,
    FlexBasis = 0x45,
    AlignItems = 0x46,
    AlignSelf = 0x47,
    AlignContent = 0x48,
    JustifyContent = 0x49,
    JustifyItems = 0x4A,
    JustifySelf = 0x4B,
    Order = 0x4C,
    
    // Taffy Positioning Properties (0x50-0x5F)
    Position = 0x50,
    Top = 0x51,
    Right = 0x52,
    Bottom = 0x53,
    Left = 0x54,
    Inset = 0x55,
    
    // Taffy Sizing Properties (0x90-0x9F)
    MinSize = 0x90,
    MaxSize = 0x91,
    PreferredSize = 0x92,
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
    
    // Taffy-specific value types
    GridTrack = 0x10,        // fr, px, %, auto units for grid tracks
    GridArea = 0x11,         // Grid area specification (line-based)
    FlexValue = 0x12,        // Flex grow/shrink values
    AlignmentValue = 0x13,   // Alignment enum values
    PositionValue = 0x14,    // Position enum (relative, absolute, etc.)
    LengthPercentage = 0x15, // CSS length-percentage values
    Dimension = 0x16,        // Auto, Length, Percentage dimension
    
    // Transform-specific value types
    Transform = 0x17,        // Transform object with multiple properties
    TransformMatrix = 0x18,  // 4x4 or 2x3 transformation matrix
    CSSUnit = 0x19,          // CSS unit values (px, em, rem, vw, vh, deg, rad, turn)
    Transform2D = 0x1A,      // Optimized 2D transform (scale, translate, rotate)
    Transform3D = 0x1B,      // Full 3D transform data
}

// Layout flags (must match renderer's LayoutDirection enum)
pub const LAYOUT_DIRECTION_MASK: u8 = 0x03;
pub const LAYOUT_DIRECTION_ROW: u8 = 0;        // Row layout
pub const LAYOUT_DIRECTION_COLUMN: u8 = 1;     // Column layout  
pub const LAYOUT_DIRECTION_ABSOLUTE: u8 = 2;   // Absolute positioning
// Note: ROW_REV and COL_REV are not supported by the renderer

pub const LAYOUT_ALIGNMENT_MASK: u8 = 0x0C;
pub const LAYOUT_ALIGNMENT_START: u8 = 0 << 2;
pub const LAYOUT_ALIGNMENT_CENTER: u8 = 1 << 2;
pub const LAYOUT_ALIGNMENT_END: u8 = 2 << 2;
pub const LAYOUT_ALIGNMENT_SPACE_BETWEEN: u8 = 3 << 2;

pub const LAYOUT_WRAP_BIT: u8 = 1 << 4;
pub const LAYOUT_GROW_BIT: u8 = 1 << 5;
pub const LAYOUT_ABSOLUTE_BIT: u8 = 1 << 6;

// Event Types
pub const EVENT_TYPE_CLICK: u8 = 0x01;
pub const EVENT_TYPE_PRESS: u8 = 0x02;
pub const EVENT_TYPE_RELEASE: u8 = 0x03;
pub const EVENT_TYPE_HOVER: u8 = 0x04;
pub const EVENT_TYPE_FOCUS: u8 = 0x05;
pub const EVENT_TYPE_BLUR: u8 = 0x06;
pub const EVENT_TYPE_CHANGE: u8 = 0x07;
pub const EVENT_TYPE_SUBMIT: u8 = 0x08;

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

// Transform data structures
#[derive(Debug, Clone)]
pub struct TransformData {
    pub transform_type: TransformType,
    pub properties: Vec<TransformProperty>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransformType {
    Transform2D = 0x01,      // Basic 2D transform
    Transform3D = 0x02,      // Full 3D transform
    Matrix2D = 0x03,         // 2x3 matrix
    Matrix3D = 0x04,         // 4x4 matrix
}

#[derive(Debug, Clone)]
pub struct TransformProperty {
    pub property_type: TransformPropertyType,
    pub value_type: ValueType,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransformPropertyType {
    // 2D Transform properties
    Scale = 0x01,
    ScaleX = 0x02,
    ScaleY = 0x03,
    TranslateX = 0x04,
    TranslateY = 0x05,
    Rotate = 0x06,
    SkewX = 0x07,
    SkewY = 0x08,
    
    // 3D Transform properties
    ScaleZ = 0x09,
    TranslateZ = 0x0A,
    RotateX = 0x0B,
    RotateY = 0x0C,
    RotateZ = 0x0D,
    Perspective = 0x0E,
    
    // Matrix properties
    Matrix = 0x0F,
}

impl TransformPropertyType {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "scale" => Some(Self::Scale),
            "scale_x" => Some(Self::ScaleX),
            "scale_y" => Some(Self::ScaleY),
            "translate_x" => Some(Self::TranslateX),
            "translate_y" => Some(Self::TranslateY),
            "rotate" => Some(Self::Rotate),
            "skew_x" => Some(Self::SkewX),
            "skew_y" => Some(Self::SkewY),
            "scale_z" => Some(Self::ScaleZ),
            "translate_z" => Some(Self::TranslateZ),
            "rotate_x" => Some(Self::RotateX),
            "rotate_y" => Some(Self::RotateY),
            "rotate_z" => Some(Self::RotateZ),
            "perspective" => Some(Self::Perspective),
            "matrix" => Some(Self::Matrix),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CSSUnitValue {
    pub value: f64,
    pub unit: CSSUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CSSUnit {
    // Size units
    Pixels = 0x01,      // px
    Em = 0x02,          // em
    Rem = 0x03,         // rem
    ViewportWidth = 0x04, // vw
    ViewportHeight = 0x05, // vh
    Percentage = 0x06,   // %
    
    // Angle units
    Degrees = 0x07,     // deg
    Radians = 0x08,     // rad
    Turns = 0x09,       // turn
    
    // Unitless (for scale, matrix values)
    Number = 0x0A,
}

impl CSSUnit {
    pub fn from_property_value(value: &crate::ast::PropertyValue) -> Option<CSSUnitValue> {
        match value {
            crate::ast::PropertyValue::Pixels(v) => Some(CSSUnitValue { value: *v, unit: CSSUnit::Pixels }),
            crate::ast::PropertyValue::Em(v) => Some(CSSUnitValue { value: *v, unit: CSSUnit::Em }),
            crate::ast::PropertyValue::Rem(v) => Some(CSSUnitValue { value: *v, unit: CSSUnit::Rem }),
            crate::ast::PropertyValue::ViewportWidth(v) => Some(CSSUnitValue { value: *v, unit: CSSUnit::ViewportWidth }),
            crate::ast::PropertyValue::ViewportHeight(v) => Some(CSSUnitValue { value: *v, unit: CSSUnit::ViewportHeight }),
            crate::ast::PropertyValue::Percentage(v) => Some(CSSUnitValue { value: *v, unit: CSSUnit::Percentage }),
            crate::ast::PropertyValue::Degrees(v) => Some(CSSUnitValue { value: *v, unit: CSSUnit::Degrees }),
            crate::ast::PropertyValue::Radians(v) => Some(CSSUnitValue { value: *v, unit: CSSUnit::Radians }),
            crate::ast::PropertyValue::Turns(v) => Some(CSSUnitValue { value: *v, unit: CSSUnit::Turns }),
            crate::ast::PropertyValue::Number(v) => Some(CSSUnitValue { value: *v, unit: CSSUnit::Number }),
            crate::ast::PropertyValue::Integer(v) => Some(CSSUnitValue { value: *v as f64, unit: CSSUnit::Number }),
            _ => None,
        }
    }
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

/// Template variable definition in KRB format
#[derive(Debug, Clone)]
pub struct TemplateVariable {
    pub name: String,
    pub name_index: u8,
    pub value_type: ValueType,
    pub default_value: String,
    pub default_value_index: u8,
}

/// Template binding that connects a property to template variables
#[derive(Debug, Clone)]
pub struct TemplateBinding {
    pub element_index: u16,
    pub property_id: u8,
    pub template_expression: String,
    pub template_expression_index: u8,
    pub variable_count: u8,
    pub variable_indices: Vec<u8>, // Indices into the template variables array
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
pub struct FontEntry {
    pub name: String,
    pub path: String,
    pub name_index: u8,
    pub path_index: u8,
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
    pub checked: bool,
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
    pub fonts: Vec<FontEntry>,
    pub scripts: Vec<ScriptEntry>,
    pub resources: Vec<ResourceEntry>,
    pub component_defs: Vec<ComponentDefinition>,
    pub component_ast_templates: HashMap<String, AstNode>,
    pub variables: HashMap<String, VariableDef>,
    pub variable_context: crate::variable_context::VariableContext,

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
    
    // Template variable data
    pub template_variables: Vec<TemplateVariable>,
    pub template_bindings: Vec<TemplateBinding>,
    pub template_variable_offset: u32,
    pub template_binding_offset: u32,
    pub total_template_variable_size: u32,
    pub total_template_binding_size: u32,
    
    // Transform data
    pub transforms: Vec<TransformData>,
    pub transform_offset: u32,
    pub total_transform_size: u32,
    
    // Function template data
    pub function_templates: Vec<FunctionTemplate>,
    pub resolved_functions: HashMap<String, ResolvedFunction>,
    pub component_functions: HashMap<String, Vec<String>>, // Component instance -> [func names]
    pub next_template_id: usize,
}

impl CompilerState {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            strings: Vec::new(),
            styles: Vec::new(),
            fonts: Vec::new(),
            scripts: Vec::new(),
            resources: Vec::new(),
            component_defs: Vec::new(),
            component_ast_templates: HashMap::new(),
            variables: HashMap::new(),
            variable_context: crate::variable_context::VariableContext::new(),
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
            template_variables: Vec::new(),
            template_bindings: Vec::new(),
            template_variable_offset: 0,
            template_binding_offset: 0,
            total_template_variable_size: 0,
            total_template_binding_size: 0,
            transforms: Vec::new(),
            transform_offset: 0,
            total_transform_size: 0,
            function_templates: Vec::new(),
            resolved_functions: HashMap::new(),
            component_functions: HashMap::new(),
            next_template_id: 0,
        }
    }
    
    /// Add a string to the string table with deduplication
    /// Returns the index of the string (existing or newly added)
    pub fn add_string<S: AsRef<str>>(&mut self, text: S) -> Result<u8, crate::error::CompilerError> {
        let text_str = text.as_ref();
        
        // Handle empty string special case
        if text_str.is_empty() {
            if self.strings.is_empty() {
                self.strings.push(StringEntry {
                    text: String::new(),
                    length: 0,
                    index: 0,
                });
            }
            return Ok(0);
        }
        
        // Check if string already exists
        for (index, existing) in self.strings.iter().enumerate() {
            if existing.text == text_str {
                return Ok(index as u8);
            }
        }
        
        // Check limits
        if self.strings.len() >= MAX_STRINGS {
            return Err(crate::error::CompilerError::LimitExceeded {
                limit_type: "strings".to_string(),
                limit: MAX_STRINGS,
            });
        }
        
        // Add new string
        let index = self.strings.len() as u8;
        self.strings.push(StringEntry {
            text: text_str.to_string(),
            length: text_str.len(),
            index,
        });
        Ok(index)
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

// Function template system for dynamic function names
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct FunctionTemplate {
    pub id: usize,
    pub name_pattern: String,        // "toggle_dropdown_$component_id"
    pub body: String,               // Function body with $vars
    pub parameters: Vec<String>,    // ["value", "index"]
    pub language: String,           // "lua"
    pub scope: FunctionScope,       // Global or Component("Dropdown")
    pub required_vars: HashSet<String>, // Variables used in name/body
    pub source_location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionScope {
    Global,
    Component(String), // Component name
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct ResolvedFunction {
    pub name: String,              // "toggle_colors"
    pub code: String,              // Body with vars substituted
    pub template_id: usize,        // Links back to template
    pub instance_context: Option<String>, // "Dropdown:colors"
    pub language: String,          // "lua"
    pub parameters: Vec<String>,   // ["value", "index"]
}