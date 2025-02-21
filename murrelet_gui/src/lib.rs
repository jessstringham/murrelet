pub use murrelet_gui_derive::MurreletGUI;

// #[derive(Debug, Error)]
// pub enum MurreletGUIErr {
//     #[error("An error occurred decoding GUI to livecode: {0}")]
//     GUIToLivecode(String),
// }

// pub type MurreletGUIResult<T> = Result<T, MurreletGUIErr>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueGUI {
    Bool,         // should be a ControlBool
    Num,          // should be a ControlF32
    Name(String), // clue for the front-end to sink the strings iwth the same name
    Color,        // expected to give h s v a
    Defs,         // make a ctx node
    Vec2,
    Vec3,
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

    pub fn list(m: MurreletGUISchema) -> Self {
        Self::List(Box::new(m))
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
