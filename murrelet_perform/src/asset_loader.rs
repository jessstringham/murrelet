use std::{collections::HashMap, path::Path};

use itertools::Itertools;
use lerpable::Lerpable;
use murrelet_common::{
    Assets, JsonAssetLookup, RasterAsset, RasterAssetLookup, VectorAsset, VectorLayersAssetLookup,
};
use murrelet_gui::CanMakeGUI;
use murrelet_livecode_derive::Livecode;

pub trait VectorAssetLoader {
    fn is_match(&self, file_extension: &str) -> bool;
    fn load(&self, layers: &[&str], filename: &Path) -> VectorAsset;
}

pub trait RasterAssetLoader {
    fn is_match(&self, file_extension: &str) -> bool;
    fn load(&self, filename: &Path) -> RasterAsset;
}

#[derive(Livecode, Lerpable, Clone, Debug)]
pub struct JsonStringFile {
    #[livecode(kind = "none")]
    name: String,
    #[livecode(kind = "none")]
    content: String,
    // probably will want to add something to normalize the nums coming in...
}
impl JsonStringFile {
    pub fn path(&self) -> &Path {
        Path::new(&self.name)
    }
}

#[derive(Livecode, Lerpable, Clone, Debug)]
pub struct RasterFile {
    #[livecode(kind = "none")]
    name: String,
    // probably will want to add something to normalize the nums coming in...
}
impl RasterFile {
    pub fn path(&self) -> &Path {
        Path::new(&self.name)
    }
}

#[derive(Livecode, Lerpable, Clone, Debug)]
pub struct PolylineLayerFile {
    #[livecode(kind = "none")]
    name: String,
    #[livecode(kind = "none")]
    layers: String,
}
impl PolylineLayerFile {
    pub fn new(name: String, layers: String) -> Self {
        Self { name, layers }
    }
    pub fn path(&self) -> &Path {
        Path::new(&self.name)
    }
}

pub fn _empty_filenames() -> ControlAssetFilenames {
    ControlAssetFilenames {
        vector_files: vec![],
        raster_files: vec![],
        json_files: vec![],
    }
}

pub fn _empty_filenames_lazy() -> ControlLazyAssetFilenames {
    ControlLazyAssetFilenames {
        vector_files: vec![],
        raster_files: vec![],
        json_files: vec![],
    }
}

pub struct AssetLoaders {
    vector: Vec<Box<dyn VectorAssetLoader>>,
    raster: Vec<Box<dyn RasterAssetLoader>>,
}

impl AssetLoaders {
    pub fn new(
        vector: Vec<Box<dyn VectorAssetLoader>>,
        raster: Vec<Box<dyn RasterAssetLoader>>,
    ) -> Self {
        Self { vector, raster }
    }

    pub fn empty() -> AssetLoaders {
        Self {
            vector: vec![],
            raster: vec![],
        }
    }
}

#[derive(Livecode, Lerpable, Clone, Debug)]
pub struct AssetFilenames {
    // hmm, the parsers are all in a different part of the code
    vector_files: Vec<PolylineLayerFile>,
    raster_files: Vec<RasterFile>,
    json_files: Vec<JsonStringFile>, // just load as a string
}

impl AssetFilenames {
    pub fn empty() -> Self {
        Self {
            vector_files: Vec::new(),
            raster_files: Vec::new(),
            json_files: Vec::new(),
        }
    }

    pub fn load_polylines(&self, load_funcs: &AssetLoaders) -> Assets {
        let mut m = HashMap::new();
        for filename in &self.vector_files {
            let path = filename.path();

            println!("loading vector file {:?}", filename.path());
            // depending on the filetype...

            if let Some(ext) = path.extension() {
                let ext_str = ext.to_str();
                for func in &load_funcs.vector {
                    if func.is_match(ext_str.unwrap()) {
                        let filename_stem = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        let layers: Vec<&str> = filename.layers.split(",").collect_vec();
                        m.insert(filename_stem, func.load(&layers, path));
                    }
                }
            }
        }

        let polylines = VectorLayersAssetLookup::new(m);

        let mut raster = RasterAssetLookup::empty();
        for filename in &self.raster_files {
            let path = filename.path();
            println!("loading raster file {:?}", filename.path());

            if let Some(ext) = path.extension() {
                let ext_str = ext.to_str();
                for func in &load_funcs.raster {
                    if func.is_match(ext_str.unwrap()) {
                        let filename_stem = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        raster.insert(filename_stem, func.load(path));
                    }
                }
            }
        }

        let mut json = JsonAssetLookup::empty();
        for s in &self.json_files {
            json.insert(s.name.clone(), s.content.clone());
        }

        Assets::new(polylines, raster, json)
    }
}

impl CanMakeGUI for AssetFilenames {
    fn make_gui() -> murrelet_gui::MurreletGUISchema {
        murrelet_gui::MurreletGUISchema::Skip
    }
}
