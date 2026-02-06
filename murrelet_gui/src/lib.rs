use itertools::Itertools;
#[cfg(feature = "murrelet")]
use murrelet_common::MurreletColor;
pub use murrelet_gui_derive::MurreletGUI;
use murrelet_schema::{MurreletEnumVal, MurreletPrimitive, MurreletSchema};
use serde::Serialize;

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
    String,
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
    Enum(String, Vec<MurreletEnumValGUI>, bool),      // type, val, is untagged
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
        if let Self::Enum(_, v, _) = self {
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

#[cfg(feature = "murrelet")]
impl CanMakeGUI for MurreletColor {
    fn make_gui() -> MurreletGUISchema {
        MurreletGUISchema::Val(ValueGUI::Color)
    }
}

#[cfg(feature = "murrelet")]
pub fn make_gui_angle() -> MurreletGUISchema {
    MurreletGUISchema::Val(ValueGUI::Angle)
}

#[cfg(feature = "murrelet")]
pub fn make_gui_vec2() -> MurreletGUISchema {
    MurreletGUISchema::Val(ValueGUI::Vec2)
}

#[cfg(feature = "murrelet")]
pub fn make_gui_vec2_coords() -> MurreletGUISchema {
    MurreletGUISchema::Val(ValueGUI::Coords)
}

#[cfg(feature = "murrelet")]
pub fn make_gui_vec3() -> MurreletGUISchema {
    MurreletGUISchema::Val(ValueGUI::Vec3)
}

// if you already have a schema, you can transform it
pub trait CanChangeToGUI: Sized {
    fn change_to_gui(&self) -> MurreletGUISchema;
}

impl CanChangeToGUI for MurreletSchema {
    fn change_to_gui(&self) -> MurreletGUISchema {
        match self {
            MurreletSchema::NewType(name, murrelet_schema) => {
                MurreletGUISchema::NewType(name.clone(), Box::new(murrelet_schema.change_to_gui()))
            }
            MurreletSchema::Struct(name, items) => MurreletGUISchema::Struct(
                name.clone(),
                items
                    .iter()
                    .map(|(k, v)| (k.clone(), v.change_to_gui()))
                    .collect::<Vec<_>>(),
            ),
            MurreletSchema::Enum(name, items, b) => MurreletGUISchema::Enum(
                name.clone(),
                items.values().map(change_enum_to_gui)
                    .collect_vec(),
                *b,
            ),
            MurreletSchema::List(murrelet_schema) => {
                MurreletGUISchema::List(Box::new(murrelet_schema.change_to_gui()))
            }
            MurreletSchema::Val(murrelet_primitive) => {
                MurreletGUISchema::Val(change_primitive_to_gui(murrelet_primitive))
            }
            MurreletSchema::Skip => MurreletGUISchema::Skip,
        }
    }
}

fn change_enum_to_gui(a: &MurreletEnumVal) -> MurreletEnumValGUI {
    match a {
        MurreletEnumVal::Unnamed(a, murrelet_schema) => {
            MurreletEnumValGUI::Unnamed(a.clone(), murrelet_schema.change_to_gui())
        }
        MurreletEnumVal::Unit(a) => MurreletEnumValGUI::Unit(a.clone()),
    }
}

fn change_primitive_to_gui(a: &MurreletPrimitive) -> ValueGUI {
    match a {
        MurreletPrimitive::Bool => ValueGUI::Bool,
        MurreletPrimitive::Num => ValueGUI::Num,
        MurreletPrimitive::Color => ValueGUI::Color,
        MurreletPrimitive::Defs => ValueGUI::Defs,
        MurreletPrimitive::Vec2 => ValueGUI::Vec2,
        MurreletPrimitive::Vec3 => ValueGUI::Vec3,
        MurreletPrimitive::Style => ValueGUI::Style,
        MurreletPrimitive::Angle => ValueGUI::Angle,
        MurreletPrimitive::Coords => ValueGUI::Coords,
        MurreletPrimitive::String => ValueGUI::String,
    }
}
