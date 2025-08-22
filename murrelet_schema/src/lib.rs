use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
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
    Unnamed(String, Box<MurreletSchema>),
    Unit(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum MurreletSchema {
    NewType(String, Box<MurreletSchema>),
    Struct(String, BTreeMap<String, MurreletSchema>),
    // hmm, we'll need to figure out how to provide this
    Enum(String, BTreeMap<String, MurreletEnumVal>, bool),
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

    pub fn as_enum(&self) -> Option<&BTreeMap<String, MurreletEnumVal>> {
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

    pub fn unwrap_to_struct_fields(self) -> BTreeMap<String, MurreletSchema> {
        match self {
            MurreletSchema::Struct(_, items) => items,
            _ => unreachable!("tried to flatten a struct that wasn't a struct"),
        }
    }

    pub fn update_with_hints(&self, s: &std::collections::HashMap<String, String>) -> Result<Self> {
        // basically we go through s and then traverse until we find the spot we need to update

        let mut n = self.clone();

        for (key, new_type) in s.iter() {
            let location = key.split(".").collect::<Vec<_>>();

            let kind = match new_type.to_lowercase().as_str() {
                "vec2" => Ok(MurreletPrimitive::Vec2),
                "f32" => Ok(MurreletPrimitive::Num),
                "number" => Ok(MurreletPrimitive::Num),
                "num" => Ok(MurreletPrimitive::Num),
                "string" => Ok(MurreletPrimitive::String),
                _ => Result::Err(anyhow!(format!("unsupported schema hint, {}", new_type))),
            }?;

            n.update_with_one_hint(key, &location, &kind)?;
        }

        Ok(n)
    }

    pub fn update_with_one_hint(
        &mut self,
        original_location: &str, // for debugging
        location: &[&str],
        value: &MurreletPrimitive,
    ) -> Result<()> {
        // basically we go through s and then traverse until we find the spot we need to update

        if location.is_empty() {
            // we're there!
            // return Result::Err(anyhow!(format!("couldn't find {} in {:?}!", original_location, self)));
            *self = MurreletSchema::Val(value.clone());
            return Ok(());
        }

        match self {
            MurreletSchema::NewType(_, murrelet_schema) => {
                murrelet_schema.update_with_one_hint(original_location, location, value)
            }
            MurreletSchema::Struct(_, items) => {
                if let Some((first_name, rest)) = location.split_first() {
                    let mut found_match = false;
                    for (name, schema) in items {
                        if name == first_name {
                            found_match = true;
                            schema.update_with_one_hint(original_location, rest, value)?;
                            break;
                        }
                    }
                    if !found_match {
                        Result::Err(anyhow!(format!("{} didn't match", first_name)))
                    } else {
                        Ok(())
                    }
                } else {
                    Result::Err(anyhow!("missing"))
                }
            }
            MurreletSchema::Enum(_, _, _) => todo!(),
            // need to do this one...
            MurreletSchema::List(_) => todo!(),
            // i think these should be handled in the struct level!
            MurreletSchema::Val(_) => todo!(),
            MurreletSchema::Skip => Result::Err(anyhow!("hm, trying to edit a skip")),
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
