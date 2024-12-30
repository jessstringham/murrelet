// simple thing that holds a mapping from a string to a list of polylines.
// perform is the thing that uses this

use std::{collections::HashMap, sync::Arc};

use glam::Vec2;
use itertools::Itertools;

use crate::{IsPolyline, Polyline};

#[derive(Debug, Clone)]
pub struct Asset(HashMap<String, Vec<Polyline>>);
impl Asset {
    pub fn get_layer(&self, layer: &str) -> Option<&Vec<Polyline>> {
        self.0.get(layer)
    }

    pub fn from_data(x: HashMap<String, Vec<Vec<Vec2>>>) -> Self {
        let mut hm = HashMap::new();
        for (k, v) in &x {
            let p = v
                .into_iter()
                .map(|x| Polyline::new(x.clone_to_vec()))
                .collect_vec();
            hm.insert(k.clone(), p);
        }
        Self(hm)
    }
}

#[derive(Debug, Clone)]
pub struct Assets {
    filename_to_polyline_layers: HashMap<String, Asset>,
}
impl Assets {
    pub fn new(filename_to_polyline_layers: HashMap<String, Asset>) -> Self {
        Self {
            filename_to_polyline_layers,
        }
    }

    pub fn empty() -> Self {
        Self {
            filename_to_polyline_layers: HashMap::new(),
        }
    }

    pub fn new_ref(filename_to_polyline_layers: HashMap<String, Asset>) -> AssetsRef {
        Arc::new(Self::new(filename_to_polyline_layers))
    }

    pub fn empty_ref() -> AssetsRef {
        Arc::new(Self::empty())
    }

    pub fn asset_layer(&self, key: &str, layer: &str) -> Option<&Vec<Polyline>> {
        if let Some(v) = self.filename_to_polyline_layers.get(key) {
            v.get_layer(layer)
        } else {
            None
        }
    }

    pub fn to_ref(self) -> AssetsRef {
        Arc::new(self)
    }
}

pub type AssetsRef = Arc<Assets>;
