//! Layered low-bit weights for Perci's native cognition experiments.
//!
//! This module is deliberately a sidecar to `PERCIW03`, not a replacement for
//! the sparse associative field.  Perci does not currently own a dense
//! trainable Transformer, so the useful piece we can implement and measure
//! now is the representation: ternary block weights, residual bit-planes,
//! INT4 activations with a sparse precision lane, reversible Hadamard
//! rotation, and a small learned-style low-rank correction path.
//!
//! The hot representation is still compact and inspectable.  Quantization is
//! performed with higher precision during construction; inference consumes the
//! ternary planes and correction factors.  Promotion into a live weight pack
//! remains an explicit, human-authorized operation.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const MAGIC: &[u8; 8] = b"PERCLBW1";
pub const VERSION: u32 = 1;

const HEADER_SIZE: usize = 64;
const Q8: f32 = 256.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LowBitError {
    EmptyInput,
    LengthMismatch { expected: usize, actual: usize },
    InvalidConfig(String),
    NonPowerOfTwo(usize),
    NonFinite { index: usize },
    InvalidBinary(String),
    Io(String),
}

impl fmt::Display for LowBitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "low-bit input must not be empty"),
            Self::LengthMismatch { expected, actual } => {
                write!(
                    f,
                    "low-bit length mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InvalidConfig(message) => write!(f, "invalid low-bit config: {message}"),
            Self::NonPowerOfTwo(length) => {
                write!(
                    f,
                    "Hadamard rotation requires a power-of-two length, got {length}"
                )
            }
            Self::NonFinite { index } => write!(f, "non-finite value at index {index}"),
            Self::InvalidBinary(message) => write!(f, "invalid PERCLBW1 field: {message}"),
            Self::Io(message) => write!(f, "low-bit I/O error: {message}"),
        }
    }
}

impl std::error::Error for LowBitError {}

/// Configuration for the layered weight representation.
///
/// The recommended production range is a 64-weight block with one or two
/// residual planes and a rank-8 correction lane.  Smaller blocks are allowed
/// for tests and tiny matrices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayeredWeightConfig {
    pub block_size: usize,
    pub residual_planes: usize,
    pub correction_rank: usize,
    pub activation_bits: u8,
}

impl Default for LayeredWeightConfig {
    fn default() -> Self {
        Self {
            block_size: 64,
            residual_planes: 2,
            correction_rank: 8,
            activation_bits: 4,
        }
    }
}

impl LayeredWeightConfig {
    pub fn validate(self) -> Result<(), LowBitError> {
        if self.block_size < 2 || self.block_size > 128 || !self.block_size.is_power_of_two() {
            return Err(LowBitError::InvalidConfig(
                "block_size must be a power of two between 2 and 128".into(),
            ));
        }
        if self.residual_planes > 3 {
            return Err(LowBitError::InvalidConfig(
                "at most three residual planes are supported".into(),
            ));
        }
        if self.correction_rank > 32 {
            return Err(LowBitError::InvalidConfig(
                "correction_rank must be at most 32".into(),
            ));
        }
        if self.activation_bits != 4 {
            return Err(LowBitError::InvalidConfig(
                "the current activation path is INT4; other widths need a new field version".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TernaryPlane {
    pub len: usize,
    /// Positive and negative sign masks are disjoint; neither bit set is zero.
    pub positive: Vec<u64>,
    pub negative: Vec<u64>,
    /// Q8.8 block scale.  This is compact and explicit rather than a naked bit.
    pub scale_q8: u16,
}

impl TernaryPlane {
    fn quantize(values: &[f32], zero_fraction: f32) -> Result<Self, LowBitError> {
        if values.is_empty() {
            return Err(LowBitError::EmptyInput);
        }
        ensure_finite(values)?;
        let mean_abs = values.iter().map(|v| v.abs()).sum::<f32>() / values.len() as f32;
        let scale = mean_abs.max(1.0 / Q8);
        let threshold = (scale * zero_fraction.max(0.0)).min(scale);
        let words = words_for(values.len());
        let mut positive = vec![0u64; words];
        let mut negative = vec![0u64; words];
        for (index, value) in values.iter().copied().enumerate() {
            if value.abs() < threshold {
                continue;
            }
            let mask = 1u64 << (index & 63);
            if value.is_sign_negative() {
                negative[index >> 6] |= mask;
            } else {
                positive[index >> 6] |= mask;
            }
        }
        Ok(Self {
            len: values.len(),
            positive,
            negative,
            scale_q8: encode_scale(scale),
        })
    }

    fn decode(&self) -> Vec<f32> {
        let scale = decode_scale(self.scale_q8);
        (0..self.len)
            .map(|index| {
                let mask = 1u64 << (index & 63);
                if self.positive[index >> 6] & mask != 0 {
                    scale
                } else if self.negative[index >> 6] & mask != 0 {
                    -scale
                } else {
                    0.0
                }
            })
            .collect()
    }

    fn dot(&self, values: &[f32]) -> Result<f32, LowBitError> {
        if values.len() != self.len {
            return Err(LowBitError::LengthMismatch {
                expected: self.len,
                actual: values.len(),
            });
        }
        ensure_finite(values)?;
        let scale = decode_scale(self.scale_q8);
        let mut result = 0.0;
        for (index, value) in values.iter().copied().enumerate() {
            let mask = 1u64 << (index & 63);
            if self.positive[index >> 6] & mask != 0 {
                result += scale * value;
            } else if self.negative[index >> 6] & mask != 0 {
                result -= scale * value;
            }
        }
        Ok(result)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TernaryBlock {
    pub len: usize,
    pub main: TernaryPlane,
    pub residuals: Vec<TernaryPlane>,
}

impl TernaryBlock {
    fn quantize(values: &[f32], residual_planes: usize) -> Result<Self, LowBitError> {
        let main = TernaryPlane::quantize(values, 0.5)?;
        let mut approximation = main.decode();
        let mut residuals = Vec::with_capacity(residual_planes);
        for _ in 0..residual_planes {
            let residual: Vec<f32> = values
                .iter()
                .zip(approximation.iter())
                .map(|(original, estimate)| original - estimate)
                .collect();
            let plane = TernaryPlane::quantize(&residual, 0.25)?;
            let correction = plane.decode();
            for (estimate, delta) in approximation.iter_mut().zip(correction) {
                *estimate += delta;
            }
            residuals.push(plane);
        }
        Ok(Self {
            len: values.len(),
            main,
            residuals,
        })
    }

    fn decode(&self) -> Vec<f32> {
        let mut out = self.main.decode();
        for plane in &self.residuals {
            for (value, delta) in out.iter_mut().zip(plane.decode()) {
                *value += delta;
            }
        }
        out
    }

    fn dot(&self, values: &[f32]) -> Result<f32, LowBitError> {
        let mut total = self.main.dot(values)?;
        for plane in &self.residuals {
            total += plane.dot(values)?;
        }
        Ok(total)
    }
}

/// One row/vector encoded as independent small blocks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayeredVector {
    pub len: usize,
    pub block_size: usize,
    pub blocks: Vec<TernaryBlock>,
}

impl LayeredVector {
    pub fn from_f32(values: &[f32], config: LayeredWeightConfig) -> Result<Self, LowBitError> {
        config.validate()?;
        if values.is_empty() {
            return Err(LowBitError::EmptyInput);
        }
        ensure_finite(values)?;
        let blocks = values
            .chunks(config.block_size)
            .map(|chunk| TernaryBlock::quantize(chunk, config.residual_planes))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            len: values.len(),
            block_size: config.block_size,
            blocks,
        })
    }

    pub fn reconstruct(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.len);
        for block in &self.blocks {
            out.extend(block.decode());
        }
        out.truncate(self.len);
        out
    }

    pub fn dot(&self, values: &[f32]) -> Result<f32, LowBitError> {
        if values.len() != self.len {
            return Err(LowBitError::LengthMismatch {
                expected: self.len,
                actual: values.len(),
            });
        }
        ensure_finite(values)?;
        self.blocks
            .iter()
            .enumerate()
            .try_fold(0.0, |sum, (index, block)| {
                let start = index * self.block_size;
                block
                    .dot(&values[start..start + block.len])
                    .map(|value| sum + value)
            })
    }

    pub fn storage_bytes(&self) -> usize {
        self.blocks
            .iter()
            .map(|block| {
                std::iter::once(&block.main)
                    .chain(block.residuals.iter())
                    .map(|plane| 12 + plane.positive.len() * 8 + plane.negative.len() * 8)
                    .sum::<usize>()
            })
            .sum()
    }
}

/// A compact matrix made of ternary blocks plus a sparse correction lane.
#[derive(Clone, Debug)]
pub struct LayeredMatrix {
    pub rows: usize,
    pub cols: usize,
    pub config: LayeredWeightConfig,
    pub vectors: Vec<LayeredVector>,
    pub correction: LowRankCorrection,
}

impl LayeredMatrix {
    pub fn from_f32(
        values: &[f32],
        rows: usize,
        cols: usize,
        config: LayeredWeightConfig,
    ) -> Result<Self, LowBitError> {
        config.validate()?;
        if rows == 0 || cols == 0 {
            return Err(LowBitError::EmptyInput);
        }
        let expected = rows
            .checked_mul(cols)
            .ok_or_else(|| LowBitError::InvalidConfig("matrix dimensions overflow".into()))?;
        if values.len() != expected {
            return Err(LowBitError::LengthMismatch {
                expected,
                actual: values.len(),
            });
        }
        ensure_finite(values)?;
        let vectors = values
            .chunks(cols)
            .map(|row| LayeredVector::from_f32(row, config))
            .collect::<Result<Vec<_>, _>>()?;
        let quantized: Vec<f32> = vectors
            .iter()
            .flat_map(LayeredVector::reconstruct)
            .collect();
        let residual: Vec<f32> = values
            .iter()
            .zip(quantized.iter())
            .map(|(original, estimate)| original - estimate)
            .collect();
        let correction = LowRankCorrection::fit(&residual, rows, cols, config.correction_rank, 4)?;
        Ok(Self {
            rows,
            cols,
            config,
            vectors,
            correction,
        })
    }

    pub fn apply(&self, input: &[f32]) -> Result<Vec<f32>, LowBitError> {
        if input.len() != self.cols {
            return Err(LowBitError::LengthMismatch {
                expected: self.cols,
                actual: input.len(),
            });
        }
        ensure_finite(input)?;
        let mut out = self
            .vectors
            .iter()
            .map(|vector| vector.dot(input))
            .collect::<Result<Vec<_>, _>>()?;
        let correction = self.correction.apply(input)?;
        for (value, delta) in out.iter_mut().zip(correction) {
            *value += delta;
        }
        Ok(out)
    }

    pub fn apply_without_correction(&self, input: &[f32]) -> Result<Vec<f32>, LowBitError> {
        if input.len() != self.cols {
            return Err(LowBitError::LengthMismatch {
                expected: self.cols,
                actual: input.len(),
            });
        }
        self.vectors
            .iter()
            .map(|vector| vector.dot(input))
            .collect()
    }

    pub fn reconstruct_quantized_weights(&self) -> Vec<f32> {
        self.vectors
            .iter()
            .flat_map(LayeredVector::reconstruct)
            .collect()
    }

    /// Reconstruct the effective matrix, including the low-rank escape lane.
    /// The correction is represented as U·Vᵀ, so probing each basis vector is
    /// exact and keeps this diagnostic path independent of a dense runtime.
    pub fn reconstruct_weights(&self) -> Vec<f32> {
        let mut out = self.reconstruct_quantized_weights();
        if self.correction.rank == 0 {
            return out;
        }
        for col in 0..self.cols {
            let mut basis = vec![0.0; self.cols];
            basis[col] = 1.0;
            if let Ok(delta) = self.correction.apply(&basis) {
                for (row, value) in delta.into_iter().enumerate() {
                    out[row * self.cols + col] += value;
                }
            }
        }
        out
    }

    pub fn weight_mse(&self, original: &[f32]) -> Result<f32, LowBitError> {
        if original.len() != self.rows * self.cols {
            return Err(LowBitError::LengthMismatch {
                expected: self.rows * self.cols,
                actual: original.len(),
            });
        }
        ensure_finite(original)?;
        let estimate = self.reconstruct_weights();
        Ok(mean_squared_error(original, &estimate))
    }

    pub fn storage_bytes(&self) -> usize {
        HEADER_SIZE
            + self
                .vectors
                .iter()
                .map(LayeredVector::storage_bytes)
                .sum::<usize>()
            + self.correction.storage_bytes()
    }

    /// Serialize the sidecar as a deterministic, mmap-friendly binary field.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![0u8; HEADER_SIZE];
        out[0..8].copy_from_slice(MAGIC);
        write_u32(&mut out, 8, VERSION);
        write_u32(&mut out, 12, self.rows as u32);
        write_u32(&mut out, 16, self.cols as u32);
        write_u16(&mut out, 20, self.config.block_size as u16);
        out[22] = self.config.residual_planes as u8;
        out[23] = self.config.activation_bits;
        write_u16(&mut out, 24, self.correction.rank as u16);
        for vector in &self.vectors {
            for block in &vector.blocks {
                write_plane(&mut out, &block.main);
                for residual in &block.residuals {
                    write_plane(&mut out, residual);
                }
            }
        }
        for value in &self.correction.left {
            out.extend_from_slice(&value.to_le_bytes());
        }
        for value in &self.correction.right {
            out.extend_from_slice(&value.to_le_bytes());
        }
        let payload_len = (out.len() - HEADER_SIZE) as u64;
        write_u64(&mut out, 28, payload_len);
        out
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, LowBitError> {
        if data.len() < HEADER_SIZE || &data[..8] != MAGIC {
            return Err(LowBitError::InvalidBinary(
                "header or magic is invalid".into(),
            ));
        }
        if read_u32(data, 8)? != VERSION {
            return Err(LowBitError::InvalidBinary("unsupported version".into()));
        }
        let rows = read_u32(data, 12)? as usize;
        let cols = read_u32(data, 16)? as usize;
        let config = LayeredWeightConfig {
            block_size: read_u16(data, 20)? as usize,
            residual_planes: data[22] as usize,
            correction_rank: read_u16(data, 24)? as usize,
            activation_bits: data[23],
        };
        let declared_payload = read_u64(data, 28)? as usize;
        if declared_payload != data.len() - HEADER_SIZE {
            return Err(LowBitError::InvalidBinary(
                "declared payload length differs from field length".into(),
            ));
        }
        config
            .validate()
            .map_err(|e| LowBitError::InvalidBinary(e.to_string()))?;
        if rows == 0 || cols == 0 {
            return Err(LowBitError::InvalidBinary("zero matrix dimension".into()));
        }
        let mut cursor = HEADER_SIZE;
        let blocks_per_row = (cols + config.block_size - 1) / config.block_size;
        let mut vectors = Vec::with_capacity(rows);
        for _ in 0..rows {
            let mut blocks = Vec::with_capacity(blocks_per_row);
            for block_index in 0..blocks_per_row {
                let expected_len = (cols - block_index * config.block_size).min(config.block_size);
                let main = read_plane(data, &mut cursor, expected_len)?;
                let mut residuals = Vec::with_capacity(config.residual_planes);
                for _ in 0..config.residual_planes {
                    residuals.push(read_plane(data, &mut cursor, expected_len)?);
                }
                blocks.push(TernaryBlock {
                    len: expected_len,
                    main,
                    residuals,
                });
            }
            vectors.push(LayeredVector {
                len: cols,
                block_size: config.block_size,
                blocks,
            });
        }
        let left_len = rows * config.correction_rank;
        let right_len = config.correction_rank * cols;
        let left = read_f32_vec(data, &mut cursor, left_len)?;
        let right = read_f32_vec(data, &mut cursor, right_len)?;
        if cursor != data.len() {
            return Err(LowBitError::InvalidBinary(
                "trailing bytes after correction lane".into(),
            ));
        }
        Ok(Self {
            rows,
            cols,
            config,
            vectors,
            correction: LowRankCorrection {
                rows,
                cols,
                rank: config.correction_rank,
                left,
                right,
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LowRankCorrection {
    pub rows: usize,
    pub cols: usize,
    pub rank: usize,
    /// Component-major U·Σ, with `rows * rank` elements.
    pub left: Vec<f32>,
    /// Row-major Vᵀ, with `rank * cols` elements.
    pub right: Vec<f32>,
}

impl LowRankCorrection {
    pub fn empty(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            rank: 0,
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    /// Fit a deterministic truncated low-rank approximation of the residual.
    /// This is a compact post-quantization correction, not silent weight
    /// promotion or a claim of gradient training.
    pub fn fit(
        residual: &[f32],
        rows: usize,
        cols: usize,
        rank: usize,
        iterations: usize,
    ) -> Result<Self, LowBitError> {
        if rows == 0 || cols == 0 || rank == 0 {
            return Ok(Self::empty(rows, cols));
        }
        if residual.len() != rows * cols {
            return Err(LowBitError::LengthMismatch {
                expected: rows * cols,
                actual: residual.len(),
            });
        }
        ensure_finite(residual)?;
        let target_rank = rank.min(rows).min(cols);
        let mut work = residual.to_vec();
        let mut left = Vec::with_capacity(rows * target_rank);
        let mut right = Vec::with_capacity(target_rank * cols);
        let mut actual_rank = 0;
        for component in 0..target_rank {
            let mut v: Vec<f32> = (0..cols)
                .map(|index| (((index + 1 + component * 17) as f32) * 0.618_034).sin())
                .collect();
            normalize(&mut v);
            let mut u = vec![0.0; rows];
            for _ in 0..iterations.max(1) {
                for row in 0..rows {
                    u[row] = (0..cols).map(|col| work[row * cols + col] * v[col]).sum();
                }
                if normalize(&mut u) == 0.0 {
                    break;
                }
                for col in 0..cols {
                    v[col] = (0..rows).map(|row| work[row * cols + col] * u[row]).sum();
                }
                if normalize(&mut v) == 0.0 {
                    break;
                }
            }
            for row in 0..rows {
                u[row] = (0..cols).map(|col| work[row * cols + col] * v[col]).sum();
            }
            let sigma = normalize(&mut u);
            if sigma <= 1.0e-7 {
                break;
            }
            left.extend(u.iter().map(|value| value * sigma));
            right.extend_from_slice(&v);
            for row in 0..rows {
                for col in 0..cols {
                    work[row * cols + col] -= sigma * u[row] * v[col];
                }
            }
            actual_rank += 1;
        }
        left.truncate(rows * actual_rank);
        right.truncate(actual_rank * cols);
        Ok(Self {
            rows,
            cols,
            rank: actual_rank,
            left,
            right,
        })
    }

    pub fn apply(&self, input: &[f32]) -> Result<Vec<f32>, LowBitError> {
        if input.len() != self.cols {
            return Err(LowBitError::LengthMismatch {
                expected: self.cols,
                actual: input.len(),
            });
        }
        ensure_finite(input)?;
        if self.rank == 0 {
            return Ok(vec![0.0; self.rows]);
        }
        let projections: Vec<f32> = (0..self.rank)
            .map(|component| {
                (0..self.cols)
                    .map(|col| self.right[component * self.cols + col] * input[col])
                    .sum()
            })
            .collect();
        Ok((0..self.rows)
            .map(|row| {
                (0..self.rank)
                    .map(|component| {
                        self.left[component * self.rows + row] * projections[component]
                    })
                    .sum()
            })
            .collect())
    }

    pub fn storage_bytes(&self) -> usize {
        self.left.len() * 4 + self.right.len() * 4
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparseOutlier {
    pub index: u32,
    /// Q8.8 escape-lane value; ordinary values remain INT4.
    pub value_q8: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuantizedActivations {
    pub bits: u8,
    pub scale_q8: u16,
    /// Stored as i8 for clarity; values are restricted to the INT4 range.
    pub low: Vec<i8>,
    pub outliers: Vec<SparseOutlier>,
}

impl QuantizedActivations {
    pub fn quantize_int4(values: &[f32]) -> Result<Self, LowBitError> {
        if values.is_empty() {
            return Err(LowBitError::EmptyInput);
        }
        ensure_finite(values)?;
        let mean_abs = values.iter().map(|value| value.abs()).sum::<f32>() / values.len() as f32;
        // A robust half-mean scale keeps ordinary signal in INT4 and sends
        // exceptional channels through the sparse lane instead of clipping.
        let scale = (mean_abs * 0.5).max(1.0 / Q8);
        let limit = scale * 7.0;
        let mut low = Vec::with_capacity(values.len());
        let mut outliers = Vec::new();
        for (index, value) in values.iter().copied().enumerate() {
            if value.abs() > limit {
                low.push(0);
                outliers.push(SparseOutlier {
                    index: index as u32,
                    value_q8: encode_signed_q8(value),
                });
            } else {
                low.push((value / scale).round().clamp(-7.0, 7.0) as i8);
            }
        }
        Ok(Self {
            bits: 4,
            scale_q8: encode_scale(scale),
            low,
            outliers,
        })
    }

    pub fn reconstruct(&self) -> Vec<f32> {
        let scale = decode_scale(self.scale_q8);
        let mut values: Vec<f32> = self.low.iter().map(|value| *value as f32 * scale).collect();
        for outlier in &self.outliers {
            if let Some(value) = values.get_mut(outlier.index as usize) {
                *value = outlier.value_q8 as f32 / Q8;
            }
        }
        values
    }

    pub fn mse(&self, original: &[f32]) -> Result<f32, LowBitError> {
        if original.len() != self.low.len() {
            return Err(LowBitError::LengthMismatch {
                expected: self.low.len(),
                actual: original.len(),
            });
        }
        ensure_finite(original)?;
        Ok(mean_squared_error(original, &self.reconstruct()))
    }

    pub fn outlier_density(&self) -> f32 {
        if self.low.is_empty() {
            0.0
        } else {
            self.outliers.len() as f32 / self.low.len() as f32
        }
    }
}

/// Orthonormal Walsh-Hadamard rotation.  Applying the same function twice is
/// the inverse, up to normal floating-point roundoff.
pub fn hadamard_rotate(values: &[f32]) -> Result<Vec<f32>, LowBitError> {
    if values.is_empty() {
        return Err(LowBitError::EmptyInput);
    }
    if !values.len().is_power_of_two() {
        return Err(LowBitError::NonPowerOfTwo(values.len()));
    }
    ensure_finite(values)?;
    let mut out = values.to_vec();
    let mut width = 1;
    while width < out.len() {
        let stride = width * 2;
        for start in (0..out.len()).step_by(stride) {
            for offset in 0..width {
                let a = out[start + offset];
                let b = out[start + offset + width];
                out[start + offset] = a + b;
                out[start + offset + width] = a - b;
            }
        }
        width = stride;
    }
    let normalizer = (out.len() as f32).sqrt();
    for value in &mut out {
        *value /= normalizer;
    }
    Ok(out)
}

pub fn hadamard_inverse(values: &[f32]) -> Result<Vec<f32>, LowBitError> {
    hadamard_rotate(values)
}

#[derive(Clone, Debug, PartialEq)]
pub struct LowBitProbe {
    pub baseline_mse: f32,
    pub corrected_mse: f32,
    pub activation_mse: f32,
    pub outliers: usize,
    pub hadamard_roundtrip_max_error: f32,
    pub serialized_bytes: usize,
    pub serialized_roundtrip_max_error: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LowBitTrainingProbe {
    pub input: Vec<f32>,
    pub target: Vec<f32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LowBitTrainingSet {
    pub schema: Option<String>,
    pub rows: usize,
    pub cols: usize,
    pub weights: Vec<f32>,
    #[serde(default)]
    pub heldout: Vec<LowBitTrainingProbe>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LowBitTrainingReport {
    pub schema: &'static str,
    pub rows: usize,
    pub cols: usize,
    pub candidate_path: PathBuf,
    pub manifest_path: PathBuf,
    pub candidate_bytes: usize,
    pub candidate_checksum_fnv1a64: String,
    pub baseline_weight_mse: f32,
    pub candidate_weight_mse: f32,
    pub heldout_probes: usize,
    pub baseline_heldout_mse: Option<f32>,
    pub candidate_heldout_mse: Option<f32>,
    pub candidate_beats_baseline: bool,
    /// Always false. A report is evidence; it is never authority to promote.
    pub promote_recommended: bool,
    pub claim_boundary: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct LowBitAssessmentReport {
    pub schema: &'static str,
    pub rows: usize,
    pub cols: usize,
    pub candidate_path: PathBuf,
    pub candidate_bytes: usize,
    pub candidate_checksum_fnv1a64: String,
    pub integrity_pass: bool,
    pub heldout_probes: usize,
    pub baseline_weight_mse: f32,
    pub candidate_weight_mse: f32,
    pub baseline_heldout_mse: Option<f32>,
    pub candidate_heldout_mse: Option<f32>,
    pub candidate_beats_baseline: bool,
    pub assessment: &'static str,
    pub claim_boundary: &'static str,
}

fn read_training_set(input_path: &Path) -> Result<LowBitTrainingSet, LowBitError> {
    let text =
        fs::read_to_string(input_path).map_err(|error| LowBitError::Io(error.to_string()))?;
    let dataset: LowBitTrainingSet = serde_json::from_str(&text)
        .map_err(|error| LowBitError::InvalidBinary(format!("training JSON: {error}")))?;
    if let Some(schema) = dataset.schema.as_deref() {
        if schema != "perci.lowbit.train.v1" {
            return Err(LowBitError::InvalidBinary(format!(
                "unsupported training schema {schema}"
            )));
        }
    }
    let expected = dataset
        .rows
        .checked_mul(dataset.cols)
        .ok_or_else(|| LowBitError::InvalidConfig("matrix dimensions overflow".into()))?;
    if dataset.weights.len() != expected {
        return Err(LowBitError::LengthMismatch {
            expected,
            actual: dataset.weights.len(),
        });
    }
    ensure_finite(&dataset.weights)?;
    Ok(dataset)
}

fn evaluate_matrix_candidate(
    dataset: &LowBitTrainingSet,
    baseline: &LayeredMatrix,
    candidate: &LayeredMatrix,
) -> Result<(f32, f32, Option<f32>, Option<f32>, bool), LowBitError> {
    let baseline_weight_mse = baseline.weight_mse(&dataset.weights)?;
    let candidate_weight_mse = candidate.weight_mse(&dataset.weights)?;
    let mut baseline_errors = Vec::new();
    let mut candidate_errors = Vec::new();
    for probe in &dataset.heldout {
        if probe.input.len() != dataset.cols {
            return Err(LowBitError::LengthMismatch {
                expected: dataset.cols,
                actual: probe.input.len(),
            });
        }
        if probe.target.len() != dataset.rows {
            return Err(LowBitError::LengthMismatch {
                expected: dataset.rows,
                actual: probe.target.len(),
            });
        }
        ensure_finite(&probe.input)?;
        ensure_finite(&probe.target)?;
        baseline_errors.push(mean_squared_error(
            &probe.target,
            &baseline.apply(&probe.input)?,
        ));
        candidate_errors.push(mean_squared_error(
            &probe.target,
            &candidate.apply(&probe.input)?,
        ));
    }
    let baseline_heldout_mse = (!baseline_errors.is_empty())
        .then(|| baseline_errors.iter().sum::<f32>() / baseline_errors.len() as f32);
    let candidate_heldout_mse = (!candidate_errors.is_empty())
        .then(|| candidate_errors.iter().sum::<f32>() / candidate_errors.len() as f32);
    let candidate_beats_baseline = match (baseline_heldout_mse, candidate_heldout_mse) {
        (Some(base), Some(candidate)) => {
            candidate <= base && candidate_weight_mse <= baseline_weight_mse
        }
        _ => candidate_weight_mse <= baseline_weight_mse,
    };
    Ok((
        baseline_weight_mse,
        candidate_weight_mse,
        baseline_heldout_mse,
        candidate_heldout_mse,
        candidate_beats_baseline,
    ))
}

/// Build a candidate PERCLBW1 field from a reviewed JSON dataset.
///
/// Expected input schema:
///
/// ```json
/// {
///   "schema": "perci.lowbit.train.v1",
///   "rows": 2,
///   "cols": 4,
///   "weights": [0.1, -0.2, 0.3, -0.4, 0.2, 0.1, -0.5, 0.7],
///   "heldout": [{"input": [1, 0, 0, 0], "target": [0.1, 0.2]}]
/// }
/// ```
///
/// The held-out probes are matrix-vector workloads.  This is a native
/// representation trainer and evaluator, not a language-model pretraining
/// loop; it deliberately produces an isolated candidate and a receipt.
pub fn train_from_json(
    input_path: impl AsRef<Path>,
    candidate_path: impl AsRef<Path>,
    config: LayeredWeightConfig,
) -> Result<LowBitTrainingReport, LowBitError> {
    let input_path = input_path.as_ref();
    let candidate_path = candidate_path.as_ref().to_path_buf();
    let dataset = read_training_set(input_path)?;

    let baseline_config = LayeredWeightConfig {
        correction_rank: 0,
        ..config
    };
    let baseline = LayeredMatrix::from_f32(
        &dataset.weights,
        dataset.rows,
        dataset.cols,
        baseline_config,
    )?;
    let candidate = LayeredMatrix::from_f32(&dataset.weights, dataset.rows, dataset.cols, config)?;
    let (
        baseline_weight_mse,
        candidate_weight_mse,
        baseline_heldout_mse,
        candidate_heldout_mse,
        candidate_beats_baseline,
    ) = evaluate_matrix_candidate(&dataset, &baseline, &candidate)?;

    let bytes = candidate.to_bytes();
    if let Some(parent) = candidate_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| LowBitError::Io(error.to_string()))?;
    }
    fs::write(&candidate_path, &bytes).map_err(|error| LowBitError::Io(error.to_string()))?;
    let manifest_path = candidate_path.with_extension("json");
    let report = LowBitTrainingReport {
        schema: "perci.lowbit.report.v1",
        rows: dataset.rows,
        cols: dataset.cols,
        candidate_path: candidate_path.clone(),
        manifest_path: manifest_path.clone(),
        candidate_bytes: bytes.len(),
        candidate_checksum_fnv1a64: format!("{:016x}", fnv1a64(&bytes)),
        baseline_weight_mse,
        candidate_weight_mse,
        heldout_probes: dataset.heldout.len(),
        baseline_heldout_mse,
        candidate_heldout_mse,
        candidate_beats_baseline,
        promote_recommended: false,
        claim_boundary:
            "representation and matrix-vector evidence only; not language-model pretraining or AGI",
    };
    let manifest = serde_json::to_vec_pretty(&report)
        .map_err(|error| LowBitError::InvalidBinary(format!("training report: {error}")))?;
    fs::write(&manifest_path, manifest).map_err(|error| LowBitError::Io(error.to_string()))?;
    Ok(report)
}

/// Re-open a candidate and independently assess it against the reviewed data.
/// This deliberately does not rebuild or rewrite the candidate, so the receipt
/// proves that the bytes on disk—not only an in-memory training object—passed.
pub fn assess_candidate_from_json(
    input_path: impl AsRef<Path>,
    candidate_path: impl AsRef<Path>,
) -> Result<LowBitAssessmentReport, LowBitError> {
    let dataset = read_training_set(input_path.as_ref())?;
    let candidate_path = candidate_path.as_ref().to_path_buf();
    let bytes = fs::read(&candidate_path).map_err(|error| LowBitError::Io(error.to_string()))?;
    let candidate = LayeredMatrix::from_bytes(&bytes)?;
    if candidate.rows != dataset.rows || candidate.cols != dataset.cols {
        return Err(LowBitError::LengthMismatch {
            expected: dataset.rows * dataset.cols,
            actual: candidate.rows * candidate.cols,
        });
    }
    let baseline_config = LayeredWeightConfig {
        correction_rank: 0,
        ..candidate.config
    };
    let baseline = LayeredMatrix::from_f32(
        &dataset.weights,
        dataset.rows,
        dataset.cols,
        baseline_config,
    )?;
    let (
        baseline_weight_mse,
        candidate_weight_mse,
        baseline_heldout_mse,
        candidate_heldout_mse,
        candidate_beats_baseline,
    ) = evaluate_matrix_candidate(&dataset, &baseline, &candidate)?;
    Ok(LowBitAssessmentReport {
        schema: "perci.lowbit.assessment.v1",
        rows: dataset.rows,
        cols: dataset.cols,
        candidate_path,
        candidate_bytes: bytes.len(),
        candidate_checksum_fnv1a64: format!("{:016x}", fnv1a64(&bytes)),
        integrity_pass: true,
        heldout_probes: dataset.heldout.len(),
        baseline_weight_mse,
        candidate_weight_mse,
        baseline_heldout_mse,
        candidate_heldout_mse,
        candidate_beats_baseline,
        assessment: if candidate_beats_baseline {
            "PASS"
        } else {
            "HOLD"
        },
        claim_boundary:
            "representation and matrix-vector evidence only; not language-model pretraining or AGI",
    })
}

/// A deterministic, small end-to-end probe for the new representation.
/// It is intentionally diagnostic; it does not write or promote weights.
pub fn run_probe() -> Result<LowBitProbe, LowBitError> {
    let values: Vec<f32> = vec![
        0.20, -0.40, 0.70, 1.10, -0.15, 0.35, 0.90, -1.20, 0.10, -0.30, 0.50, 1.30, -0.25, 0.45,
        0.80, -1.00, 0.30, -0.60, 0.95, 1.20, -0.20, 0.40, 0.65, -1.10,
    ];
    let config = LayeredWeightConfig {
        block_size: 4,
        residual_planes: 2,
        correction_rank: 2,
        activation_bits: 4,
    };
    let baseline = LayeredMatrix::from_f32(
        &values,
        4,
        6,
        LayeredWeightConfig {
            correction_rank: 0,
            ..config
        },
    )?;
    let model = LayeredMatrix::from_f32(&values, 4, 6, config)?;
    let baseline_mse = baseline.weight_mse(&values)?;
    let corrected_weights = model.reconstruct_weights();
    let corrected_mse = mean_squared_error(&values, &corrected_weights);
    let activations = [0.03, -0.06, 0.08, 0.02, 14.7, -0.11, 0.04, 0.09];
    let quantized = QuantizedActivations::quantize_int4(&activations)?;
    let activation_mse = quantized.mse(&activations)?;
    let rotated = hadamard_rotate(&activations)?;
    let restored = hadamard_inverse(&rotated)?;
    let hadamard_roundtrip_max_error = activations
        .iter()
        .zip(restored.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max);
    let bytes = model.to_bytes();
    let restored_model = LayeredMatrix::from_bytes(&bytes)?;
    let input = [0.2, -0.3, 0.5, 0.7, -0.4, 0.1];
    let before = model.apply(&input)?;
    let after = restored_model.apply(&input)?;
    let serialized_roundtrip_max_error = before
        .iter()
        .zip(after.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max);
    Ok(LowBitProbe {
        baseline_mse,
        corrected_mse,
        activation_mse,
        outliers: quantized.outliers.len(),
        hadamard_roundtrip_max_error,
        serialized_bytes: bytes.len(),
        serialized_roundtrip_max_error,
    })
}

pub fn status_report() -> String {
    let config = LayeredWeightConfig::default();
    format!(
        "layered low-bit sidecar\n  format: PERCLBW1 v{}\n  weights: ternary {{-1,0,+1}} + Q8.8 scale per {}-weight block\n  residuals: up to {} ternary correction planes\n  activations: INT{} with sparse Q8.8 outlier lane\n  rotation: orthonormal Walsh-Hadamard (reversible)\n  correction: rank-{} low-rank residual path\n  authority: experimental sidecar; PERCIW03 remains active and weight promotion is human-gated",
        VERSION,
        config.block_size,
        config.residual_planes,
        config.activation_bits,
        config.correction_rank
    )
}

fn words_for(len: usize) -> usize {
    (len + 63) / 64
}

fn ensure_finite(values: &[f32]) -> Result<(), LowBitError> {
    for (index, value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(LowBitError::NonFinite { index });
        }
    }
    Ok(())
}

fn encode_scale(scale: f32) -> u16 {
    (scale.clamp(1.0 / Q8, u16::MAX as f32 / Q8) * Q8).round() as u16
}

fn decode_scale(scale_q8: u16) -> f32 {
    (scale_q8.max(1) as f32) / Q8
}

fn encode_signed_q8(value: f32) -> i16 {
    (value.clamp(i16::MIN as f32 / Q8, i16::MAX as f32 / Q8) * Q8).round() as i16
}

fn mean_squared_error(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() {
        return 0.0;
    }
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        / left.len() as f32
}

fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn normalize(values: &mut [f32]) -> f32 {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in values {
            *value /= norm;
        }
    }
    norm
}

fn write_u16(out: &mut Vec<u8>, offset: usize, value: u16) {
    if out.len() < offset + 2 {
        out.resize(offset + 2, 0);
    }
    out[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, offset: usize, value: u32) {
    if out.len() < offset + 4 {
        out.resize(offset + 4, 0);
    }
    out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut Vec<u8>, offset: usize, value: u64) {
    if out.len() < offset + 8 {
        out.resize(offset + 8, 0);
    }
    out[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_plane(out: &mut Vec<u8>, plane: &TernaryPlane) {
    out.extend_from_slice(&(plane.len as u32).to_le_bytes());
    out.extend_from_slice(&plane.scale_q8.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(plane.positive.len() as u32).to_le_bytes());
    for word in &plane.positive {
        out.extend_from_slice(&word.to_le_bytes());
    }
    for word in &plane.negative {
        out.extend_from_slice(&word.to_le_bytes());
    }
}

fn read_plane(
    data: &[u8],
    cursor: &mut usize,
    expected_len: usize,
) -> Result<TernaryPlane, LowBitError> {
    let len = read_u32_at(data, cursor)? as usize;
    let scale_q8 = read_u16_at(data, cursor)?;
    let _reserved = read_u16_at(data, cursor)?;
    let word_count = read_u32_at(data, cursor)? as usize;
    let expected_words = words_for(len);
    if len != expected_len || word_count != expected_words {
        return Err(LowBitError::InvalidBinary(
            "plane dimensions do not match the matrix".into(),
        ));
    }
    let mut positive = Vec::with_capacity(word_count);
    let mut negative = Vec::with_capacity(word_count);
    for _ in 0..word_count {
        positive.push(read_u64_at(data, cursor)?);
    }
    for _ in 0..word_count {
        negative.push(read_u64_at(data, cursor)?);
    }
    Ok(TernaryPlane {
        len,
        positive,
        negative,
        scale_q8,
    })
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, LowBitError> {
    data.get(offset..offset + 2)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| LowBitError::InvalidBinary("truncated u16".into()))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, LowBitError> {
    data.get(offset..offset + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| LowBitError::InvalidBinary("truncated u32".into()))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64, LowBitError> {
    data.get(offset..offset + 8)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(|| LowBitError::InvalidBinary("truncated u64".into()))
}

fn read_u64_at(data: &[u8], cursor: &mut usize) -> Result<u64, LowBitError> {
    let value = data
        .get(*cursor..*cursor + 8)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(|| LowBitError::InvalidBinary("truncated u64".into()))?;
    *cursor += 8;
    Ok(value)
}

fn read_u16_at(data: &[u8], cursor: &mut usize) -> Result<u16, LowBitError> {
    let value = data
        .get(*cursor..*cursor + 2)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or_else(|| LowBitError::InvalidBinary("truncated u16".into()))?;
    *cursor += 2;
    Ok(value)
}

fn read_u32_at(data: &[u8], cursor: &mut usize) -> Result<u32, LowBitError> {
    let value = data
        .get(*cursor..*cursor + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| LowBitError::InvalidBinary("truncated u32".into()))?;
    *cursor += 4;
    Ok(value)
}

fn read_f32_vec(data: &[u8], cursor: &mut usize, count: usize) -> Result<Vec<f32>, LowBitError> {
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let bits = read_u32_at(data, cursor)?;
        let value = f32::from_bits(bits);
        if !value.is_finite() {
            return Err(LowBitError::InvalidBinary(
                "non-finite correction value".into(),
            ));
        }
        values.push(value);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ternary_plane_preserves_zero_and_sign() {
        let plane = TernaryPlane::quantize(&[0.0, 2.0, -2.0, 0.1], 0.5).unwrap();
        let decoded = plane.decode();
        assert_eq!(decoded[0], 0.0);
        assert!(decoded[1] > 0.0);
        assert!(decoded[2] < 0.0);
        assert_eq!(decoded[3], 0.0);
    }

    #[test]
    fn residual_planes_reduce_weight_error() {
        let values = [0.13, -0.47, 0.91, 1.27, -0.22, 0.38, 0.73, -1.11];
        let one = LayeredVector::from_f32(
            &values,
            LayeredWeightConfig {
                block_size: 8,
                residual_planes: 0,
                correction_rank: 0,
                activation_bits: 4,
            },
        )
        .unwrap();
        let three = LayeredVector::from_f32(
            &values,
            LayeredWeightConfig {
                block_size: 8,
                residual_planes: 2,
                correction_rank: 0,
                activation_bits: 4,
            },
        )
        .unwrap();
        let mse_one = mean_squared_error(&values, &one.reconstruct());
        let mse_three = mean_squared_error(&values, &three.reconstruct());
        assert!(mse_three < mse_one, "{mse_three} should beat {mse_one}");
    }

    #[test]
    fn low_rank_correction_reduces_matrix_error() {
        let values: Vec<f32> = (0..24)
            .map(|index| ((index as f32) * 0.37).sin() * 0.8)
            .collect();
        let baseline = LayeredMatrix::from_f32(
            &values,
            4,
            6,
            LayeredWeightConfig {
                block_size: 4,
                residual_planes: 1,
                correction_rank: 0,
                activation_bits: 4,
            },
        )
        .unwrap();
        let corrected = LayeredMatrix::from_f32(
            &values,
            4,
            6,
            LayeredWeightConfig {
                block_size: 4,
                residual_planes: 1,
                correction_rank: 2,
                activation_bits: 4,
            },
        )
        .unwrap();
        assert!(corrected.correction.rank > 0);
        let baseline_error = baseline.weight_mse(&values).unwrap();
        let corrected_error = corrected.weight_mse(&values).unwrap();
        assert!(
            corrected_error < baseline_error,
            "{corrected_error} >= {baseline_error}"
        );
    }

    #[test]
    fn int4_uses_sparse_precision_lane_for_outliers() {
        let values = [0.03, -0.06, 0.08, 0.02, 14.7];
        let quantized = QuantizedActivations::quantize_int4(&values).unwrap();
        assert_eq!(quantized.bits, 4);
        assert_eq!(quantized.outliers.len(), 1);
        assert!(quantized.mse(&values).unwrap() < 0.01);
    }

    #[test]
    fn hadamard_rotation_is_reversible() {
        let values = [0.03, -0.06, 0.08, 0.02, 14.7, -0.11, 0.04, 0.09];
        let rotated = hadamard_rotate(&values).unwrap();
        let restored = hadamard_inverse(&rotated).unwrap();
        let error = values
            .iter()
            .zip(restored.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f32::max);
        assert!(error < 1.0e-5, "roundtrip error {error}");
    }

    #[test]
    fn binary_sidecar_roundtrips_exactly() {
        let values: Vec<f32> = (0..24)
            .map(|index| ((index as f32) * 0.23).cos() * 0.7)
            .collect();
        let config = LayeredWeightConfig {
            block_size: 4,
            residual_planes: 2,
            correction_rank: 2,
            activation_bits: 4,
        };
        let model = LayeredMatrix::from_f32(&values, 4, 6, config).unwrap();
        let bytes = model.to_bytes();
        assert_eq!(&bytes[..8], MAGIC);
        let restored = LayeredMatrix::from_bytes(&bytes).unwrap();
        let input = [0.2, -0.3, 0.5, 0.7, -0.4, 0.1];
        let before = model.apply(&input).unwrap();
        let after = restored.apply(&input).unwrap();
        for (left, right) in before.iter().zip(after.iter()) {
            assert!((left - right).abs() < 1.0e-6);
        }
    }

    #[test]
    fn training_pipeline_writes_candidate_and_manifest_without_promotion() {
        let root =
            std::env::temp_dir().join(format!("perci-lowbit-training-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let input = root.join("dataset.json");
        let output = root.join("candidate.blw");
        let dataset = r#"{
            "schema": "perci.lowbit.train.v1",
            "rows": 2,
            "cols": 4,
            "weights": [0.1, -0.2, 0.3, -0.4, 0.2, 0.1, -0.5, 0.7],
            "heldout": [
                {"input": [1.0, 0.0, 0.0, 0.0], "target": [0.1, 0.2]},
                {"input": [0.0, 0.0, 1.0, -1.0], "target": [0.7, -1.2]}
            ]
        }"#;
        std::fs::write(&input, dataset).unwrap();
        let report = train_from_json(
            &input,
            &output,
            LayeredWeightConfig {
                block_size: 4,
                residual_planes: 2,
                correction_rank: 2,
                activation_bits: 4,
            },
        )
        .unwrap();
        assert!(output.is_file());
        assert!(output.with_extension("json").is_file());
        assert!(!report.promote_recommended);
        assert!(report.candidate_bytes > HEADER_SIZE);
        assert_eq!(report.heldout_probes, 2);
        let bytes = std::fs::read(&output).unwrap();
        assert_eq!(LayeredMatrix::from_bytes(&bytes).unwrap().rows, 2);
        let assessed = assess_candidate_from_json(&input, &output).unwrap();
        assert!(assessed.integrity_pass);
        assert_eq!(assessed.assessment, "PASS");
        assert_eq!(assessed.candidate_bytes, report.candidate_bytes);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn probe_is_deterministic_and_measurable() {
        let first = run_probe().unwrap();
        let second = run_probe().unwrap();
        assert_eq!(first, second);
        assert!(first.outliers > 0);
        assert!(first.hadamard_roundtrip_max_error < 1.0e-5);
        assert!(first.serialized_roundtrip_max_error < 1.0e-6);
        assert!(first.serialized_bytes > HEADER_SIZE);
    }
}
