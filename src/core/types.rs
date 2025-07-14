// FILE: src/core/types.rs

// Element Types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ElementType {
    App = 0x00,
    Container = 0x01,
    Text = 0x02,
    Link = 0x03,
    Image = 0x04,
    Canvas = 0x05,
    WasmView = 0x06,
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
            "Link" => Self::Link,
            "Image" => Self::Image,
            "Canvas" => Self::Canvas,
            "WasmView" => Self::WasmView,
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
