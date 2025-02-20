pub use murrelet_gui_derive::MurreletGUI;
use murrelet_livecode::livecode::ControlF32;
use thiserror::Error;

// #[derive(Debug, Error)]
// pub enum MurreletGUIErr {
//     #[error("An error occurred decoding GUI to livecode: {0}")]
//     GUIToLivecode(String),
// }

// pub type MurreletGUIResult<T> = Result<T, MurreletGUIErr>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueGUI {
    Bool,
    Num,
    Name(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MurreletEnumValGUI {
    Unnamed(String, MurreletGUISchema),
    Unit(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MurreletGUISchema {
    Val(ValueGUI),
    NewType(Box<MurreletGUISchema>),
    Struct(Vec<(String, MurreletGUISchema)>), // field val
    List(Box<MurreletGUISchema>),
    Enum(Vec<MurreletEnumValGUI>), // type, val
    Skip,
}
impl MurreletGUISchema {
    pub fn new_type(m: MurreletGUISchema) -> Self {
        Self::NewType(Box::new(m))
    }

    pub fn as_enum(&self) -> Option<&Vec<MurreletEnumValGUI>> {
        if let Self::Enum(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_new_type(&self) -> Option<&Box<MurreletGUISchema>> {
        if let Self::NewType(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

// this should be on the Control version
pub trait CanMakeGUI: Sized {
    fn make_gui() -> MurreletGUISchema;

    // fn gui_to_livecode(&self, gui_val: MurreletGUIResponse) -> MurreletGUIResult<Self>;
}


impl CanMakeGUI for f32 {
    fn make_gui() -> MurreletGUISchema {
        MurreletGUISchema::Val(ValueGUI::Num)
    }
}

impl<T: CanMakeGUI> CanMakeGUI for Vec<T> {
    fn make_gui() -> MurreletGUISchema {

        // blargh
        MurreletGUISchema::List(Box::new(T::make_gui()))
    }
}