use itertools::Itertools;
use lerpable::Lerpable;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rand_distr::Normal;
use std::{collections::HashSet, fmt};

#[derive(Debug, PartialEq)]
pub enum EmbeddingError {
    DecodeEmpty,
    DecodeHasWrongDigitCount { expected: usize, got: usize },
    DecodeValueOutOfRange { max: u32, got: u32 },
    // InvalidValue(String),
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Lerpable)]
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
            .zip(rns.into_iter())
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
}

impl From<&[f32]> for MurreletQuantizedEmbedding {
    fn from(value: &[f32]) -> Self {
        MurreletQuantizedEmbedding::from_rn(value, 9)
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

#[derive(Clone, Debug)]
pub struct MurreletEmbeddingConf {
    digits: usize,
    len: usize,
}

impl MurreletEmbeddingConf {
    pub fn new_high_limit(len: usize) -> Self {
        Self { digits: 9, len }
    }

    pub fn new(digits: usize, len: usize) -> Self {
        Self { digits, len }
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
    conf: MurreletEmbeddingConf,
}
impl EmbeddingGenCommand {
    pub fn new(steps: EmbeddingGenStep, conf: MurreletEmbeddingConf) -> Self {
        Self { steps, conf }
    }

    pub fn compute(&self) -> MurreletQuantizedEmbedding {
        self.steps.compute(&self.conf)
    }
}

#[derive(Clone, Debug)]
pub enum EmbeddingGenStep {
    Seed(u64),
    NearSeedGaussian {
        source: Box<EmbeddingGenStep>,
        rand_seed: u64, // seed for gaussian
        stdev: f32,
    },
    NearSeedReRand {
        source: Box<EmbeddingGenStep>,
        rand_seed: u64,     // seed for rerand
        rerand_chance: f32, // controls how many are updated, per item
    },
    Mix {
        source_a: Box<EmbeddingGenStep>,
        source_b: Box<EmbeddingGenStep>,
        mix: f32,
    },
}
impl EmbeddingGenStep {
    pub fn compute(&self, conf: &MurreletEmbeddingConf) -> MurreletQuantizedEmbedding {
        match &self {
            EmbeddingGenStep::Seed(seed) => MurreletQuantizedEmbedding::from_seed(&conf, *seed),
            EmbeddingGenStep::NearSeedGaussian {
                source,
                rand_seed,
                stdev,
            } => {
                let base = source.compute(conf);
                base.new_with_gaussian_noise(*rand_seed, *stdev)
            }
            EmbeddingGenStep::NearSeedReRand {
                source,
                rand_seed,
                rerand_chance,
            } => {
                let base = source.compute(conf);
                base.new_with_rerandomize(*rand_seed, *rerand_chance)
            }
            EmbeddingGenStep::Mix {
                source_a,
                source_b,
                mix,
            } => {
                let base_a = source_a.compute(conf);
                let base_b = source_b.compute(conf);
                base_a.lerpify(&base_b, mix)
            }
        }
    }

    pub(crate) fn with_conf(&self, conf: &MurreletEmbeddingConf) -> EmbeddingGenCommand {
        EmbeddingGenCommand::new(self.clone(), conf.clone())
    }
}
