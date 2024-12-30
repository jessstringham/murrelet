use std::{collections::HashMap, path::Path};

use itertools::Itertools;
use murrelet_common::assets::{Asset, Assets};
use murrelet_livecode_derive::Livecode;

pub trait AssetLoader {
    fn is_match(&self, file_extension: &str) -> bool;
    fn load(&self, filename: &Path) -> Asset;
}

#[derive(Livecode, Clone, Debug)]
pub struct PolylineLayerFile {
    #[livecode(kind = "none")]
    name: String,
}
impl PolylineLayerFile {
    pub fn new(name: String) -> Self {
        Self { name }
    }
    pub fn path(&self) -> &Path {
        Path::new(&self.name)
    }
}



pub fn _empty_filenames() -> ControlAssetFilenames {
    ControlAssetFilenames { files: vec![] }
}

pub fn _empty_filenames_lazy() -> ControlLazyAssetFilenames {
    ControlLazyAssetFilenames { files: vec![] }
}

#[derive(Livecode, Clone, Debug)]
pub struct AssetFilenames {
    // hmm, the parsers are all in a different part of the code
    files: Vec<PolylineLayerFile>,
}

impl AssetFilenames {
    pub fn new(files: Vec<String>) -> Self {
        Self {
            files: files
                .into_iter()
                .map(|x| PolylineLayerFile::new(x))
                .collect_vec()
        }
    }

    pub fn empty() -> Self {
        Self {
            files: Vec::new(),
        }
    }

    pub fn load(&self, load_funcs: &[Box<dyn AssetLoader>]) -> Assets {
        let mut m = HashMap::new();
        for filename in &self.files {
            let path = filename.path();

            println!("loading file {:?}", filename.path());
            // depending on the filetype...

            if let Some(ext) = path.extension() {
                let ext_str = ext.to_str();
                for func in load_funcs {
                    if func.is_match(ext_str.unwrap()) {

                        let filename_stem = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                        m.insert(filename_stem, func.load(path));
                    }
                }
            }
        }

        Assets::new(m)
    }
}
