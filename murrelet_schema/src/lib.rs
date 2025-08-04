use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MurreletPrimitive {
    Bool,   // should be a ControlBool
    Num,    // should be a ControlF32
    Color,  // expected to give h s v a
    Defs,   // make a ctx node
    Vec2,   // arbitrary vec2, also see Coords
    Vec3,   // arbitrary vec3
    Style,  // murrelet style
    Angle,  // angle pi
    Coords, // global coords, so the user can click things
    String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MurreletEnumVal {
    Unnamed(String, MurreletSchema),
    Unit(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MurreletSchema {
    NewType(String, Box<MurreletSchema>),
    Struct(String, Vec<(String, MurreletSchema)>),
    Enum(String, Vec<MurreletEnumVal>, bool),
    List(Box<MurreletSchema>),
    Val(MurreletPrimitive),
    Skip,
}
impl MurreletSchema {
    pub fn new_type(name: String, m: MurreletSchema) -> Self {
        Self::NewType(name, Box::new(m))
    }

    pub fn list(m: MurreletSchema) -> Self {
        Self::List(Box::new(m))
    }

    pub fn as_enum(&self) -> Option<&Vec<MurreletEnumVal>> {
        if let Self::Enum(_, v, _) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_new_type(&self) -> Option<&Box<MurreletSchema>> {
        if let Self::NewType(_, v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn unwrap_to_struct_fields(self) -> Vec<(String, MurreletSchema)> {
        match self {
            MurreletSchema::Struct(_, items) => items,
            _ => unreachable!("tried to flatten a struct that wasn't a struct"),
        }
    }
}

// this should be on the Control version
pub trait CanMakeSchema: Sized {
    fn make_schema() -> MurreletSchema;
}

macro_rules! impl_can_make_schema_for_num {
    ($ty:ty) => {
        impl CanMakeSchema for $ty {
            fn make_schema() -> MurreletSchema {
                MurreletSchema::Val(MurreletPrimitive::Num)
            }
        }
    };
}

impl_can_make_schema_for_num!(f32);
impl_can_make_schema_for_num!(f64);
impl_can_make_schema_for_num!(u32);
impl_can_make_schema_for_num!(u64);
impl_can_make_schema_for_num!(i32);
impl_can_make_schema_for_num!(i64);
impl_can_make_schema_for_num!(usize);

impl<T: CanMakeSchema> CanMakeSchema for Vec<T> {
    fn make_schema() -> MurreletSchema {
        MurreletSchema::List(Box::new(T::make_schema()))
    }
}

impl CanMakeSchema for String {
    fn make_schema() -> MurreletSchema {
        MurreletSchema::Val(MurreletPrimitive::String)
    }
}

impl CanMakeSchema for bool {
    fn make_schema() -> MurreletSchema {
        MurreletSchema::Val(MurreletPrimitive::Bool)
    }
}

#[cfg(feature = "glam")]
impl CanMakeSchema for glam::Vec2 {
    fn make_schema() -> MurreletSchema {
        MurreletSchema::Val(MurreletPrimitive::Vec2)
    }
}

#[cfg(feature = "glam")]
impl CanMakeSchema for glam::Vec3 {
    fn make_schema() -> MurreletSchema {
        MurreletSchema::Val(MurreletPrimitive::Vec3)
    }
}
