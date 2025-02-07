// simple thing that holds a mapping from a string to a list of polylines.
// perform is the thing that uses this

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use glam::Vec2;
use itertools::Itertools;

use crate::{IsPolyline, Polyline};

// eeh, this is not very good, svg assumptions are all mixed in
#[derive(Debug, Clone)]
pub struct Asset {
    layers: Vec<String>, // to support indexing
    map: HashMap<String, Vec<Polyline>>,
}
impl Asset {
    pub fn get_layer(&self, layer: &str) -> Option<&Vec<Polyline>> {
        self.map.get(layer)
    }

    pub fn layer_name_from_idx(&self, layer_idx: usize) -> &str {
        &self.layers[layer_idx % self.layers.len()]
    }

    pub fn get_layer_idx(&self, layer_id: usize) -> Option<&Vec<Polyline>> {
        let layer_name = self.layer_name_from_idx(layer_id);
        self.map.get(layer_name)
    }

    pub fn from_data(layers: Vec<String>, x: HashMap<String, Vec<Vec<Vec2>>>) -> Self {
        let layer_names: HashSet<_> = layers.iter().collect();
        let key_names: HashSet<_> = x.keys().collect();
        if layer_names != key_names {
            assert_eq!(layer_names, key_names);
        }

        let mut hm = HashMap::new();
        for (k, v) in &x {
            let p = v
                .into_iter()
                .map(|x| Polyline::new(x.clone_to_vec()))
                .collect_vec();
            hm.insert(k.clone(), p);
        }
        Self { layers, map: hm }
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

    pub fn asset_layer(&self, key: &str, layer_idx: usize) -> Option<&Vec<Polyline>> {
        let asset = &self.filename_to_polyline_layers[key];
        asset.get_layer_idx(layer_idx)
    }

    pub fn to_ref(self) -> AssetsRef {
        Arc::new(self)
    }

    pub fn layer_for_key(&self, key: &str) -> &[String] {
        &self.filename_to_polyline_layers[key].layers
    }
}

pub type AssetsRef = Arc<Assets>;
