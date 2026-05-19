pub mod embedding;

use crate::embedding::EmbeddingGenStep;
use crate::embedding::MurreletEmbeddingConf;
use crate::embedding::MurreletQuantizedEmbedding;
pub use murrelet_gen_derive::MurreletGen;

// like names, but includes
#[derive(Clone, Debug, serde::Serialize)]
pub struct RnSpec {
    pub method: String,
    pub name: String,
    pub params: Vec<(String, f32)>,
}
impl RnSpec {
    pub fn new(method: &str, name: &str, params: Vec<(String, f32)>) -> Self {
        Self {
            method: method.to_string(),
            name: name.to_string(),
            params,
        }
    }
}

pub trait CanSampleFromDist: Sized {
    // returns the right number of rn needed to generate this.
    fn rn_count() -> usize;
    fn rn_names() -> Vec<String>;

    // one RnSpec per rn, parallel to rn_names() — the gen method + params.
    fn rn_specs() -> Vec<RnSpec>;

    fn sample_dist(rn: &[f32], start_idx: usize) -> Self;

    // given rn of length ^, it'll generate!
    fn from_slice(rn: &[f32]) -> Self {
        Self::sample_dist(rn, 0)
    }

    fn from_dist<Emb>(rn: Emb) -> Self
    where
        Emb: Into<MurreletQuantizedEmbedding>,
    {
        let rnf32: MurreletQuantizedEmbedding = rn.into();
        Self::sample_dist(&rnf32.as_rn(), 0)
    }

    fn conf_high_limit() -> MurreletEmbeddingConf {
        MurreletEmbeddingConf::new_high_limit(Self::rn_count())
    }

    // usually you'll call this one, or use the MurreletQuantizedEmbedding or its DSL, esp if you need custom quantizing
    fn gen_from_seed(seed: u64) -> Self {
        let cmd = EmbeddingGenStep::Seed(seed).with_conf(&Self::conf_high_limit());
        Self::from_dist(cmd)
    }

    // should map back to itself
    fn to_dist(&self) -> Vec<f32>;
    fn to_dist_mask(&self) -> Vec<bool>;
}

pub fn prefix_field_names(prefix: String, names: Vec<String>) -> Vec<String> {
    names
        .into_iter()
        .map(|s| format!("{}.{}", prefix, s))
        .collect()
}

