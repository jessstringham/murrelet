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
pub struct VectorAsset {
    layers: Vec<String>, // to support indexing
    map: HashMap<String, Vec<Polyline>>,
}
impl VectorAsset {
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
                .iter()
                .map(|x| Polyline::new(x.clone_to_vec()))
                .collect_vec();
            hm.insert(k.clone(), p);
        }
        Self { layers, map: hm }
    }
}

#[derive(Debug, Clone)]
pub struct VectorLayersAssetLookup {
    filename_to_polyline_layers: HashMap<String, VectorAsset>,
}
impl VectorLayersAssetLookup {
    pub fn new(filename_to_polyline_layers: HashMap<String, VectorAsset>) -> Self {
        Self {
            filename_to_polyline_layers,
        }
    }

    pub fn empty() -> Self {
        Self {
            filename_to_polyline_layers: HashMap::new(),
        }
    }

    pub fn asset_layer(&self, key: &str, layer_idx: usize) -> Option<&Vec<Polyline>> {
        let asset = &self.filename_to_polyline_layers[key];
        asset.get_layer_idx(layer_idx)
    }

    pub fn layer_for_key(&self, key: &str) -> &[String] {
        &self.filename_to_polyline_layers[key].layers
    }
}

pub trait IsColorType {}

#[derive(Debug, Clone, Copy)]
pub struct BlackWhite(bool);
impl IsColorType for BlackWhite {}

// struct RGBAu8([u8; 4]),;
// struct RGBAf32([f32; 4]);

#[derive(Debug, Clone)]
pub enum RasterAsset {
    RasterBW(Vec<Vec<BlackWhite>>),
}

#[derive(Debug, Clone)]
pub struct RasterAssetLookup(HashMap<String, RasterAsset>);
impl RasterAssetLookup {
    pub fn empty() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, filename: String, img: RasterAsset) {
        self.0.insert(filename, img);
    }
}

#[derive(Debug, Clone)]

pub struct Assets {
    vectors: VectorLayersAssetLookup,
    rasters: RasterAssetLookup,
}
impl Assets {
    pub fn new(vectors: VectorLayersAssetLookup, rasters: RasterAssetLookup) -> Self {
        Self { vectors, rasters }
    }

    pub fn empty_ref() -> AssetsRef {
        Arc::new(Self::empty())
    }

    pub fn empty() -> Assets {
        Self {
            vectors: VectorLayersAssetLookup::empty(),
            rasters: RasterAssetLookup::empty(),
        }
    }

    pub fn to_ref(self) -> AssetsRef {
        Arc::new(self)
    }

    pub fn asset_layer(&self, key: &str, layer_idx: usize) -> Option<&Vec<Polyline>> {
        self.vectors.asset_layer(key, layer_idx)
    }

    pub fn layer_for_key(&self, key: &str) -> &[String] {
        self.vectors.layer_for_key(key)
    }
}

pub type AssetsRef = Arc<Assets>;
