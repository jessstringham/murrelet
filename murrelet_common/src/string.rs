use std::fmt;

use lerpable::{IsLerpingMethod, Lerpable};
use serde::{Deserialize, Serialize};

// a newtype around String so livecode can give it format-string support
// (see ControlMurreletString in murrelet_livecode). plain `String` stays a
// literal passthrough; MurreletString is the opt-in formatted version.
#[derive(Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MurreletString(String);

impl fmt::Debug for MurreletString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MurreletString({:?})", self.0)
    }
}

impl fmt::Display for MurreletString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl MurreletString {
    pub fn new(s: impl Into<String>) -> Self {
        MurreletString(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn as_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for MurreletString {
    fn from(s: String) -> Self {
        MurreletString(s)
    }
}

impl From<&str> for MurreletString {
    fn from(s: &str) -> Self {
        MurreletString(s.to_string())
    }
}

impl From<MurreletString> for String {
    fn from(s: MurreletString) -> Self {
        s.0
    }
}

// strings don't interpolate; snap to self
impl Lerpable for MurreletString {
    fn lerpify<T: IsLerpingMethod>(&self, _other: &Self, _method: &T) -> Self {
        self.clone()
    }
}
