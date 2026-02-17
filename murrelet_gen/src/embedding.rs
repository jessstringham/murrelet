use itertools::Itertools;
use lerpable::{IsLerpingMethod, Lerpable};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rand_distr::Normal;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
};

// we're going to do a u32 with 10^digit count, so this is basically the max anyway
const MAX_DIGIT_COUNT: usize = 9;

#[derive(Debug, PartialEq)]
pub enum EmbeddingError {
    DecodeEmpty,
    DecodeHasWrongDigitCount { expected: usize, got: usize },
    DecodeValueOutOfRange { max: u32, got: u32 },
    DecodeParseError(String),
}

impl fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmbeddingError::DecodeHasWrongDigitCount { expected, got } => {
                write!(
                    f,
                    "Decode invalid digit count: expected {}, got {}",
                    expected, got
                )
            }
            EmbeddingError::DecodeParseError(msg) => write!(f, "Decode parse error: {}", msg),
            EmbeddingError::DecodeEmpty => write!(f, "Decode error: empty"),
            EmbeddingError::DecodeValueOutOfRange { max, got } => {
                write!(f, "Decode digit too large: max {}, got {}", max, got)
            }
        }
    }
}

impl std::error::Error for EmbeddingError {}

pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

// hm this might be the only one you get in web actually...
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MurreletEmbeddingEncoded(String);
impl MurreletEmbeddingEncoded {
    pub fn from_decoded(d: &MurreletQuantizedEmbedding) -> EmbeddingResult<Self> {
        d.encode()
    }

    pub fn decode(&self) -> EmbeddingResult<MurreletQuantizedEmbedding> {
        MurreletQuantizedEmbedding::from_str(&self.0)
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }

    pub fn from_str(s: &str) -> EmbeddingResult<Self> {
        // we go through decoded so that we know it's valid!
        MurreletQuantizedEmbedding::from_str(s)?.encode()
    }
}

#[derive(Debug, Clone, Lerpable, Hash, PartialEq, Eq)]
pub struct MurreletQuantizedEmbedding {
    emb: Vec<u32>,
    digits: usize,
}

impl MurreletQuantizedEmbedding {
    pub fn new(emb: Vec<u32>, digits: usize) -> Self {
        Self { emb, digits }
    }

    pub fn as_rn(&self) -> Vec<f32> {
        self.emb
            .iter()
            .map(|x| *x as f32 / self.factor() as f32)
            .collect()
    }

    fn factor(&self) -> u32 {
        10u32.pow(self.digits() as u32)
    }

    fn max_val(&self) -> u32 {
        self.factor() - 1
    }

    pub fn from_seed(conf: &MurreletEmbeddingConf, seed: u64) -> MurreletQuantizedEmbedding {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let quantized: Vec<u32> = (0..conf.len)
            .map(|_| conf.compute_one_value(&mut rng))
            .collect();

        MurreletQuantizedEmbedding::new(quantized, conf.digits)
    }

    pub fn new_with_gaussian_noise(&self, seed: u64, stdev: f32) -> MurreletQuantizedEmbedding {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let base = self.as_rn();
        let rns = normal_n(&mut rng, 0.0, stdev, self.dims());

        let v = base
            .into_iter()
            .zip(rns)
            .map(|(a, b)| a + b)
            .collect::<Vec<_>>();

        MurreletQuantizedEmbedding::from_rn(&v, self.digits())
    }

    pub fn new_with_rerandomize(
        &self,
        seed: u64,
        rerand_chance: f32,
    ) -> MurreletQuantizedEmbedding {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let v = self
            .emb
            .iter()
            .map(|x| {
                if rng.gen_range(0.0..1.0) < rerand_chance {
                    rng.gen_range(0..self.factor())
                } else {
                    *x
                }
            })
            .collect_vec();

        MurreletQuantizedEmbedding::new(v, self.digits())
    }

    pub fn new_with_rerandomize_idx(
        &self,
        seed: u64,
        idx: HashSet<usize>,
    ) -> EmbeddingResult<MurreletQuantizedEmbedding> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let v = self
            .emb
            .iter()
            .enumerate()
            .map(|(i, x)| {
                if idx.contains(&i) {
                    rng.gen_range(0..self.factor())
                } else {
                    *x
                }
            })
            .collect_vec();

        Ok(MurreletQuantizedEmbedding::new(v, self.digits()))
    }

    pub fn from_rn(v: &[f32], digits: usize) -> MurreletQuantizedEmbedding {
        let digits = digits.min(MAX_DIGIT_COUNT);
        let d = quantize(v, digits);
        Self::new(d, digits)
    }

    pub fn digits(&self) -> usize {
        self.digits
    }

    pub fn dims(&self) -> usize {
        self.emb.len()
    }

    pub fn encode(&self) -> EmbeddingResult<MurreletEmbeddingEncoded> {
        let mut result = Vec::with_capacity(self.dims());

        for v in &self.emb {
            let val = *v;
            if val > self.max_val() {
                return Err(EmbeddingError::DecodeValueOutOfRange {
                    max: self.max_val(),
                    got: val,
                });
            }
            result.push(format!("{:0width$}", val, width = self.digits()));
        }

        Ok(MurreletEmbeddingEncoded(result.join("-")))
    }

    pub fn from_str(s: &str) -> EmbeddingResult<Self> {
        if s.is_empty() {
            return Err(EmbeddingError::DecodeEmpty);
        }

        let raw: Vec<&str> = s.split('-').collect();

        let digits = raw[0].len(); // know there's at least one item because otherwise we'd have returned
        let max_val = 10u32.pow(digits as u32);

        let mut decoded = Vec::with_capacity(raw.len());
        for part in raw {
            // each should have the same number of digits
            if part.len() != digits {
                return Err(EmbeddingError::DecodeHasWrongDigitCount {
                    expected: digits,
                    got: part.len(),
                });
            }

            let v = part
                .parse::<u32>()
                .map_err(|e| EmbeddingError::DecodeParseError(e.to_string()))?;

            if v >= max_val {
                return Err(EmbeddingError::DecodeValueOutOfRange {
                    max: max_val,
                    got: v,
                });
            }

            decoded.push(v);
        }

        Ok(MurreletQuantizedEmbedding::new(decoded, digits))
    }

    pub fn zero(&self, conf: &MurreletEmbeddingConf) -> Self {
        let a = vec![0; conf.len];
        Self::new(a, conf.digits)
    }

    pub fn set(&self, idx: usize, val: f32) -> Self {
        let mut v = self.clone();
        if let Some(a) = v.emb.get_mut(idx) {
            *a = quantize_one(val, self.digits());
        }
        v
    }

    pub fn dist_l2_rms(&self, other: &Self) -> Option<f32> {
        if self.digits != other.digits || self.dims() != other.dims() {
            return None;
        }
        let inv = 1.0 / self.factor() as f32;
        let mut acc = 0.0f32;
        for (&x, &y) in self.emb.iter().zip(&other.emb) {
            let d = x.abs_diff(y) as f32 * inv;
            acc += d * d;
        }
        Some((acc / self.dims() as f32).sqrt())
    }

    pub fn dist_l1_mean(&self, other: &Self) -> Option<f32> {
        if self.digits != other.digits || self.dims() != other.dims() {
            return None;
        }
        let inv = 1.0 / self.factor() as f32;
        let mut acc = 0.0f32;
        for (&x, &y) in self.emb.iter().zip(&other.emb) {
            acc += x.abs_diff(y) as f32 * inv;
        }
        Some(acc / self.dims() as f32)
    }

    pub fn dist_cosine(&self, other: &Self) -> Option<f32> {
        if self.digits() != other.digits() {
            return None;
        }
        let mut dot = 0.0f32;
        let mut na = 0.0f32;
        let mut nb = 0.0f32;
        for (&x, &y) in self.emb.iter().zip(&other.emb) {
            let ax = x as f32;
            let by = y as f32;
            dot += ax * by;
            na += ax * ax;
            nb += by * by;
        }
        let cos = dot / (na.sqrt() * nb.sqrt()).max(1e-12);

        let unit_l2 = (2.0 - 2.0 * cos).max(0.0).sqrt();

        Some(unit_l2)
    }
}

impl From<&[f32]> for MurreletQuantizedEmbedding {
    fn from(value: &[f32]) -> Self {
        MurreletQuantizedEmbedding::from_rn(value, MAX_DIGIT_COUNT)
    }
}

impl From<EmbeddingGenCommand> for MurreletQuantizedEmbedding {
    fn from(value: EmbeddingGenCommand) -> Self {
        value.compute()
    }
}

impl From<&MurreletQuantizedEmbedding> for Vec<f32> {
    fn from(value: &MurreletQuantizedEmbedding) -> Self {
        value.as_rn()
    }
}

impl From<MurreletQuantizedEmbedding> for Vec<f32> {
    fn from(value: MurreletQuantizedEmbedding) -> Self {
        value.as_rn()
    }
}

fn normal_n<R: Rng>(rng: &mut R, mean: f32, stdev: f32, count: usize) -> Vec<f32> {
    let normal = Normal::new(mean as f64, stdev as f64).unwrap();
    (0..count).map(|_| rng.sample(normal) as f32).collect()
}

fn quantize_one(decoded: f32, digits: usize) -> u32 {
    let factor = 10_u32.pow(digits as u32);
    let approx = decoded * factor as f32;
    let rounded = approx.round().max(0.0) as u32;
    let max_val = factor - 1; // need it to be max digits long, so 1 is changed to 999...
    rounded.min(max_val)
}

fn quantize(decoded: &[f32], digits: usize) -> Vec<u32> {
    let mut v = vec![];
    for i in decoded.iter() {
        let d = quantize_one(*i, digits);
        v.push(d);
    }
    v
}

#[derive(Debug, Clone)]
pub struct MemoizedEmbeddingGenerator {
    conf: MurreletEmbeddingConf,
    cache: EmbeddingGenStepCacheRef,
}
impl MemoizedEmbeddingGenerator {
    pub fn new(digits: usize, len: usize) -> Self {
        Self {
            conf: MurreletEmbeddingConf::new(digits, len),
            cache: EmbeddingGenStepCacheRef::new(),
        }
    }

    fn lookup(&self, key: &EmbeddingGenStep) -> Option<MurreletQuantizedEmbedding> {
        self.cache.0.borrow().cache.get(key).cloned()
    }

    fn insert(&self, key: &EmbeddingGenStep, val: &MurreletQuantizedEmbedding) {
        self.cache
            .0
            .borrow_mut()
            .cache
            .insert(key.clone(), val.clone());
    }
}

#[derive(Clone, Debug)]
pub struct MurreletEmbeddingConf {
    digits: usize,
    len: usize,
}

impl MurreletEmbeddingConf {
    pub fn new_high_limit(len: usize) -> Self {
        Self {
            digits: MAX_DIGIT_COUNT,
            len,
        }
    }

    pub fn new(digits: usize, len: usize) -> Self {
        Self { digits, len }
    }

    pub fn to_memoized(&self) -> MemoizedEmbeddingGenerator {
        MemoizedEmbeddingGenerator {
            conf: self.clone(),
            cache: EmbeddingGenStepCacheRef::new(),
        }
    }

    fn factor(&self) -> u32 {
        10u32.pow(self.digits as u32)
    }

    fn compute_one_value<R: Rng>(&self, rng: &mut R) -> u32 {
        rng.gen_range(0..self.factor())
    }
}

#[derive(Clone, Debug)]
pub struct EmbeddingGenCommand {
    steps: EmbeddingGenStep,
    memoized: MemoizedEmbeddingGenerator,
}
impl EmbeddingGenCommand {
    pub fn new(steps: EmbeddingGenStep, conf: MurreletEmbeddingConf) -> Self {
        Self {
            steps,
            memoized: conf.to_memoized(),
        }
    }

    pub fn compute(&self) -> MurreletQuantizedEmbedding {
        self.steps.compute(&self.memoized)
    }
}

#[derive(Debug, Clone)]
struct EmbeddingGenStepCache {
    cache: HashMap<EmbeddingGenStep, MurreletQuantizedEmbedding>,
}
impl EmbeddingGenStepCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    // pub fn compute(&mut self, step: &EmbeddingGenStep) -> &MurreletQuantizedEmbedding {
    //     if !self.cache.contains_key(step) {
    //         let c = step.compute(&self.conf);
    //         self.cache.insert(step.clone(), c);
    //     }

    //     &self.cache[step]
    // }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum EmbeddingGenStep {
    Emb(MurreletQuantizedEmbedding),
    Seed(u64),
    NearSeedGaussian {
        source: Box<EmbeddingGenStep>,
        rand_seed: u64, // seed for gaussian
        stdev: HashableF32,
    },
    NearSeedReRand {
        source: Box<EmbeddingGenStep>,
        rand_seed: u64,             // seed for rerand
        rerand_chance: HashableF32, // controls how many are updated, per item
    },
    Mix {
        source_a: Box<EmbeddingGenStep>,
        source_b: Box<EmbeddingGenStep>,
        mix: HashableF32,
    },
}
impl EmbeddingGenStep {
    pub fn seed(seed: u64) -> Self {
        Self::Seed(seed)
    }

    // useful if you edited a embedding
    pub fn emb(emb: MurreletQuantizedEmbedding) -> Self {
        Self::Emb(emb)
    }

    pub fn near_seed_gaussian(source: EmbeddingGenStep, rand_seed: u64, stdev: f32) -> Self {
        let source = Box::new(source);
        Self::NearSeedGaussian {
            source,
            rand_seed,
            stdev: stdev.into(),
        }
    }

    pub fn near_seed_rerand(source: EmbeddingGenStep, rand_seed: u64, rerand_chance: f32) -> Self {
        let source = Box::new(source);
        Self::NearSeedReRand {
            source,
            rand_seed,
            rerand_chance: rerand_chance.into(),
        }
    }

    pub fn compute(&self, mconf: &MemoizedEmbeddingGenerator) -> MurreletQuantizedEmbedding {
        if let Some(c) = mconf.lookup(self) {
            return c;
        }

        let c = self.evaluate_compute(mconf);
        mconf.insert(self, &c);
        c
    }

    pub fn evaluate_compute(
        &self,
        mconf: &MemoizedEmbeddingGenerator,
    ) -> MurreletQuantizedEmbedding {
        let conf = &mconf.conf;
        match &self {
            EmbeddingGenStep::Seed(seed) => MurreletQuantizedEmbedding::from_seed(conf, *seed),
            EmbeddingGenStep::NearSeedGaussian {
                source,
                rand_seed,
                stdev,
            } => {
                let base = source.compute(mconf);
                base.new_with_gaussian_noise(*rand_seed, stdev.into())
            }
            EmbeddingGenStep::NearSeedReRand {
                source,
                rand_seed,
                rerand_chance,
            } => {
                let base = source.compute(mconf);
                base.new_with_rerandomize(*rand_seed, rerand_chance.into())
            }
            EmbeddingGenStep::Mix {
                source_a,
                source_b,
                mix,
            } => {
                let base_a = source_a.compute(mconf);
                let base_b = source_b.compute(mconf);
                base_a.lerpify(&base_b, mix)
            }
            EmbeddingGenStep::Emb(murrelet_quantized_embedding) => {
                murrelet_quantized_embedding.clone()
            }
        }
    }

    pub(crate) fn with_conf(&self, conf: &MurreletEmbeddingConf) -> EmbeddingGenCommand {
        EmbeddingGenCommand::new(self.clone(), conf.clone())
    }
}

#[derive(Debug, Clone)]
struct EmbeddingGenStepCacheRef(Rc<RefCell<EmbeddingGenStepCache>>);
impl EmbeddingGenStepCacheRef {
    fn new() -> Self {
        Self(Rc::new(RefCell::new(EmbeddingGenStepCache::new())))
    }
}

const HASHABLE_F32_DIGITS: i32 = 6;

#[derive(Debug, Copy, Clone, Hash, PartialEq, PartialOrd, Eq)]
pub struct HashableF32(i64);

impl HashableF32 {
    #[inline]
    fn scale() -> f32 {
        10f32.powi(HASHABLE_F32_DIGITS)
    }
}

impl From<f32> for HashableF32 {
    fn from(value: f32) -> Self {
        if !value.is_finite() {
            return HashableF32(0); // fall back to 0...
        }

        let scaled = (value * Self::scale()).round() as i64;
        HashableF32(scaled)
    }
}

impl From<HashableF32> for f32 {
    fn from(value: HashableF32) -> Self {
        value.0 as f32 / HashableF32::scale()
    }
}

impl From<&HashableF32> for f32 {
    fn from(value: &HashableF32) -> Self {
        value.0 as f32 / HashableF32::scale()
    }
}

impl IsLerpingMethod for HashableF32 {
    fn has_lerp_stepped(&self) -> bool {
        let a: f32 = self.into();
        a.has_lerp_stepped()
    }

    fn partial_lerp_pct(&self, i: usize, total: usize) -> f64 {
        let a: f32 = self.into();
        a.partial_lerp_pct(i, total)
    }

    fn lerp_pct(&self) -> f64 {
        let a: f32 = self.into();
        a.lerp_pct()
    }

    fn with_lerp_pct(&self, pct: f64) -> Self {
        let a: f32 = self.into();
        a.with_lerp_pct(pct).into()
    }
}

// hm now let's make a web version
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum DeserEmbeddingGenStep {
    Emb(MurreletEmbeddingEncoded),
    Seed(u64),
    NearSeedGaussian {
        source: Box<DeserEmbeddingGenStep>,
        rand_seed: u64,
        stdev: f32,
    },
    NearSeedReRand {
        source: Box<DeserEmbeddingGenStep>,
        rand_seed: u64,
        rerand_chance: f32,
    },
    Mix {
        source_a: Box<DeserEmbeddingGenStep>,
        source_b: Box<DeserEmbeddingGenStep>,
        mix: f32,
    },
}

impl DeserEmbeddingGenStep {
    pub fn to_embedding_gen(&self) -> EmbeddingResult<EmbeddingGenStep> {
        let result = match self {
            DeserEmbeddingGenStep::Seed(a) => EmbeddingGenStep::Seed(*a),
            DeserEmbeddingGenStep::NearSeedGaussian {
                source,
                rand_seed,
                stdev,
            } => EmbeddingGenStep::NearSeedGaussian {
                source: Box::new(source.to_embedding_gen()?),
                rand_seed: *rand_seed,
                stdev: (*stdev).into(),
            },
            DeserEmbeddingGenStep::NearSeedReRand {
                source,
                rand_seed,
                rerand_chance,
            } => EmbeddingGenStep::NearSeedReRand {
                source: Box::new(source.to_embedding_gen()?),
                rand_seed: *rand_seed,
                rerand_chance: (*rerand_chance).into(),
            },
            DeserEmbeddingGenStep::Mix {
                source_a,
                source_b,
                mix,
            } => EmbeddingGenStep::Mix {
                source_a: Box::new(source_a.to_embedding_gen()?),
                source_b: Box::new(source_b.to_embedding_gen()?),
                mix: (*mix).into(),
            },
            DeserEmbeddingGenStep::Emb(emb) => EmbeddingGenStep::Emb(emb.decode()?),
        };
        Ok(result)
    }
}
