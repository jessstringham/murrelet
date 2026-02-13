use itertools::Itertools;
use rand::Rng;
use rand_distr::Normal;
use std::{collections::HashSet, fmt};

#[derive(Debug, PartialEq)]
pub enum EmbeddingError {
    DecodeInvalidLength { expected: usize, got: usize },
    DecodeHasWrongDigitCount { expected: usize, got: usize },
    InvalidValue(String),
    DecodeParseError(String),
}

impl fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmbeddingError::DecodeInvalidLength { expected, got } => {
                write!(f, "Invalid length: expected {}, got {}", expected, got)
            }
            EmbeddingError::DecodeHasWrongDigitCount { expected, got } => {
                write!(f, "Invalid digit count: expected {}, got {}", expected, got)
            }
            EmbeddingError::InvalidValue(msg) => write!(f, "Invalid value: {}", msg),
            EmbeddingError::DecodeParseError(msg) => write!(f, "Decode error: {}", msg),
        }
    }
}

impl std::error::Error for EmbeddingError {}

pub type EmbeddingResult<T> = Result<T, EmbeddingError>;

// need to create these with a MurreletEmbedding, which should be the same one used for decoding/etc
#[derive(Debug, Clone)]
pub struct MurreletEmbeddingDecoded(Vec<u32>, usize);

impl MurreletEmbeddingDecoded {
    pub fn as_rn(&self) -> Vec<f32> {
        self.0
            .iter()
            .map(|x| *x as f32 / 1.0f32.powi(self.1 as i32))
            .collect()
    }

    fn factor(&self) -> u32 {
        10u32.pow(self.digits() as u32)
    }

    pub fn new_rn(conf: &MurreletEmbeddingConf) -> MurreletEmbeddingDecoded {
        let mut rng = rand::thread_rng();

        let quantized: Vec<u32> = (0..conf.len)
            .map(|_| conf.compute_one_value(&mut rng))
            .collect();

        MurreletEmbeddingDecoded(quantized, conf.digits)
    }

    pub fn new_with_gaussian_noise(&self, stdev: f32) -> EmbeddingResult<MurreletEmbeddingDecoded> {
        let mut rng = rand::thread_rng();

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
        rerand_chance: f32,
    ) -> EmbeddingResult<MurreletEmbeddingDecoded> {
        let mut rng = rand::thread_rng();

        let v = self
            .0
            .iter()
            .map(|x| {
                if rng.gen_range(0.0..1.0) > rerand_chance {
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
        idx: HashSet<usize>,
    ) -> EmbeddingResult<MurreletEmbeddingDecoded> {
        let mut rng = rand::thread_rng();

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
            result.push(format!("{:0width$}", val, width = self.digits()));
        }

        Ok(MurreletEmbeddingEncoded(result.join("-")))
    }

    pub fn from_encoded(&self, s: &str) -> EmbeddingResult<Self> {
        let raw: Vec<&str> = if s.is_empty() {
            vec![]
        } else {
            s.split('-').collect()
        };

        let mut decoded = Vec::with_capacity(raw.len());
        for part in raw {
            // each should be digits long
            if part.len() != self.digits() {
                return Err(EmbeddingError::DecodeHasWrongDigitCount {
                    expected: self.digits(),
                    got: part.len(),
                });
            }

            let v = part
                .parse::<u32>()
                .map_err(|e| EmbeddingError::DecodeParseError(e.to_string()))?;

            decoded.push(v);
        }

        Ok(MurreletEmbeddingDecoded(decoded, self.digits()))
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

#[derive(Debug, Clone)]
pub struct MurreletEmbeddingEncoded(String);

fn quantize_one(decoded: f32, digits: usize) -> u32 {
    let max_val = 10_i32.pow(digits as u32);
    let d = (decoded * max_val as f32).round() as i32;
    let d = d.min(max_val as i32).max(0) as u32;
    d
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
        rng.gen_range(0..=self.factor())
    }
}
