pub use murrelet_gen_derive::MurreletGen;

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

pub trait CanSampleFromDist: Sized {
    // returns the right number of rn needed to generate this.
    fn rn_count() -> usize;
    fn rn_names() -> Vec<String>;

    // given rn of length ^, it'll generate!
    fn sample_dist(rn: &[f32], start_idx: usize) -> Self;

    // usually you'll call this one
    fn gen_from_seed(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);

        let rns: Vec<f32> = (0..Self::rn_count())
            .into_iter()
            .map(|_| rng.gen())
            .collect();

        Self::sample_dist(&rns, 0)
    }

    // creates an arbitrary floats that should turn back into the same values
    fn to_dist(&self) -> Vec<f32>;
    fn to_dist_mask(&self) -> Vec<bool>;
}

pub fn prefix_field_names(prefix: String, names: Vec<String>) -> Vec<String> {
    names
        .into_iter()
        .map(|s| format!("{}.{}", prefix, s))
        .collect()
}
