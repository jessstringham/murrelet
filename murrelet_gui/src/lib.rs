pub use murrelet_gui_derive::MurreletGUI;
use serde::Serialize;

// #[derive(Debug, Error)]
// pub enum MurreletGUIErr {
//     #[error("An error occurred decoding GUI to livecode: {0}")]
//     GUIToLivecode(String),
// }

// pub type MurreletGUIResult<T> = Result<T, MurreletGUIErr>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ValueGUI {
    Bool,               // should be a ControlBool
    Num,                // should be a ControlF32
    Name(String, bool), // clue for the front-end to sync the strings, bool is if it's def
    Color,              // expected to give h s v a
    Defs,               // make a ctx node
    Vec2,               // arbitrary vec2, also see Coords
    Vec3,               // arbitrary vec3
    Style,              // murrelet style
    Angle,              // angle pi
    Coords,             // global coords, so the user can click things
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MurreletEnumValGUI {
    Unnamed(String, MurreletGUISchema),
    Unit(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MurreletGUISchema {
    NewType(String, Box<MurreletGUISchema>),
    Struct(String, Vec<(String, MurreletGUISchema)>), // field val
    Enum(String, Vec<MurreletEnumValGUI>),            // type, val
    List(Box<MurreletGUISchema>),
    Val(ValueGUI),
    Skip,
}
impl MurreletGUISchema {
    pub fn new_type(name: String, m: MurreletGUISchema) -> Self {
        Self::NewType(name, Box::new(m))
    }

    pub fn list(m: MurreletGUISchema) -> Self {
        Self::List(Box::new(m))
    }

    pub fn as_enum(&self) -> Option<&Vec<MurreletEnumValGUI>> {
        if let Self::Enum(_, v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_new_type(&self) -> Option<&Box<MurreletGUISchema>> {
        if let Self::NewType(_, v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn unwrap_to_struct_fields(self) -> Vec<(String, MurreletGUISchema)> {
        match self {
            MurreletGUISchema::Struct(_, items) => items,
            _ => unreachable!("tried to flatten a struct that wasn't a struct"),
        }
    }
}

// this should be on the Control version
pub trait CanMakeGUI: Sized {
    fn make_gui() -> MurreletGUISchema;
}

macro_rules! impl_can_make_gui_for_num {
    ($ty:ty) => {
        impl CanMakeGUI for $ty {
            fn make_gui() -> MurreletGUISchema {
                MurreletGUISchema::Val(ValueGUI::Num)
            }
        }
    };
}

impl_can_make_gui_for_num!(f32);
impl_can_make_gui_for_num!(f64);
impl_can_make_gui_for_num!(u32);
impl_can_make_gui_for_num!(u64);
impl_can_make_gui_for_num!(i32);
impl_can_make_gui_for_num!(i64);
impl_can_make_gui_for_num!(usize);

impl<T: CanMakeGUI> CanMakeGUI for Vec<T> {
    fn make_gui() -> MurreletGUISchema {
        MurreletGUISchema::List(Box::new(T::make_gui()))
    }
}

impl CanMakeGUI for String {
    fn make_gui() -> MurreletGUISchema {
        MurreletGUISchema::Skip
    }
}

impl CanMakeGUI for bool {
    fn make_gui() -> MurreletGUISchema {
        MurreletGUISchema::Val(ValueGUI::Bool)
    }
}

#[cfg(feature = "glam")]
impl CanMakeGUI for glam::Vec2 {
    fn make_gui() -> MurreletGUISchema {
        MurreletGUISchema::Val(ValueGUI::Vec2)
    }
}

#[cfg(feature = "glam")]
impl CanMakeGUI for glam::Vec3 {
    fn make_gui() -> MurreletGUISchema {
        MurreletGUISchema::Val(ValueGUI::Vec3)
    }
}
