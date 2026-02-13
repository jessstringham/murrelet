use itertools::Itertools;
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
    pub fn from_decoded(d: &MurreletEmbeddingDecoded) -> EmbeddingResult<Self> {
        d.encode()
    }

    pub fn decode(&self) -> EmbeddingResult<MurreletEmbeddingDecoded> {
        MurreletEmbeddingDecoded::from_str(&self.0)
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }

    pub fn from_str(s: &str) -> EmbeddingResult<Self> {
        // we go through decoded so that we know it's valid!
        MurreletEmbeddingDecoded::from_str(s)?.encode()
    }
}

#[derive(Debug, Clone)]
pub struct MurreletEmbeddingDecoded(Vec<u32>, usize);

impl MurreletEmbeddingDecoded {
    pub fn as_rn(&self) -> Vec<f32> {
        self.0
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

    pub fn new_rn(seed: u64, conf: &MurreletEmbeddingConf) -> MurreletEmbeddingDecoded {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let quantized: Vec<u32> = (0..conf.len)
            .map(|_| conf.compute_one_value(&mut rng))
            .collect();

        MurreletEmbeddingDecoded(quantized, conf.digits)
    }

    pub fn new_with_gaussian_noise(
        &self,
        seed: u64,
        stdev: f32,
    ) -> EmbeddingResult<MurreletEmbeddingDecoded> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let base = self.as_rn();
        let rns = normal_n(&mut rng, 0.0, stdev, self.0.len());

        let v = base
            .into_iter()
            .zip(rns.into_iter())
            .map(|(a, b)| a + b)
            .collect::<Vec<_>>();

        Ok(MurreletEmbeddingDecoded::from_rn(&v, self.digits()))
    }

    pub fn new_with_rerandomize(
        &self,
        seed: u64,
        rerand_chance: f32,
    ) -> EmbeddingResult<MurreletEmbeddingDecoded> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let v = self
            .0
            .iter()
            .map(|x| {
                if rng.gen_range(0.0..1.0) < rerand_chance {
                    rng.gen_range(0..self.factor())
                } else {
                    *x
                }
            })
            .collect_vec();

        Ok(MurreletEmbeddingDecoded(v, self.digits()))
    }

    pub fn new_with_rerandomize_idx(
        &self,
        seed: u64,
        idx: HashSet<usize>,
    ) -> EmbeddingResult<MurreletEmbeddingDecoded> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        let v = self
            .0
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

        Ok(MurreletEmbeddingDecoded(v, self.digits()))
    }

    pub fn from_rn(v: &[f32], digits: usize) -> MurreletEmbeddingDecoded {
        let d = quantize(v, digits);
        Self(d, digits)
    }

    pub fn digits(&self) -> usize {
        self.1
    }

    pub fn encode(&self) -> EmbeddingResult<MurreletEmbeddingEncoded> {
        let mut result = Vec::with_capacity(self.0.len());

        for v in &self.0 {
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

        Ok(MurreletEmbeddingDecoded(decoded, digits))
    }

    pub fn zero(&self, conf: &MurreletEmbeddingConf) -> Self {
        let a = vec![0; conf.len];
        Self(a, conf.digits)
    }

    pub fn length(&self) -> usize {
        self.0.len()
    }

    pub fn set(&self, idx: usize, val: f32) -> Self {
        let mut v = self.clone();
        if let Some(a) = v.0.get_mut(idx) {
            *a = quantize_one(val, self.digits());
        }
        v
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
