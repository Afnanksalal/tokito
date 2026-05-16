//! CAD tool modes for the schematic editor.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CanvasTool {
    #[default]
    Select,
    PlaceSymbol,
    Wire,
    NetLabel,
    SheetPort,
    Power,
    Junction,
    NoConnect,
    Bus,
    Text,
    Pan,
}

impl CanvasTool {
    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::PlaceSymbol => "Place",
            Self::Wire => "Wire",
            Self::NetLabel => "Net Label",
            Self::SheetPort => "Sheet Port",
            Self::Power => "Power",
            Self::Junction => "Junction",
            Self::NoConnect => "No Connect",
            Self::Bus => "Bus",
            Self::Text => "Text",
            Self::Pan => "Pan",
        }
    }
}
