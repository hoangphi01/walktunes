// SPDX-License-Identifier: GPL-3.0-or-later
//! **wt-dsp** — pure SIMD numeric kernels with runtime CPU dispatch
//! (EC2 compute-optimized / Graviton; `docs/ARCHITECTURE.md` §3.18, ADR-0013).
//!
//! Skeleton scope (M0): detecting which SIMD path this CPU supports. FFT/STFT
//! kernels arrive with the DSP spike (R-004) and M3. This crate never does I/O.

use std::fmt;

use serde::Serialize;

/// Environment variable forcing a code path in tests and benchmarks.
pub const ENV_FORCE_SIMD: &str = "WT_FORCE_SIMD";

/// The SIMD code path selected for DSP kernels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SimdPath {
    /// x86-64 with AVX2 and FMA.
    Avx2Fma,
    /// x86-64 with SSE4.1.
    Sse41,
    /// AArch64 NEON (always present on ARMv8-A).
    Neon,
    /// Portable scalar code (RISC-V and anything else).
    Scalar,
}

impl SimdPath {
    /// Short name, as accepted by `WT_FORCE_SIMD`.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Avx2Fma => "avx2",
            Self::Sse41 => "sse41",
            Self::Neon => "neon",
            Self::Scalar => "scalar",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "avx2" => Some(Self::Avx2Fma),
            "sse41" => Some(Self::Sse41),
            "neon" => Some(Self::Neon),
            "scalar" => Some(Self::Scalar),
            _ => None,
        }
    }
}

impl fmt::Display for SimdPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The best path the CPU supports, ignoring any override.
#[must_use]
pub fn detected_simd_path() -> SimdPath {
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") && std::arch::is_x86_feature_detected!("fma")
        {
            return SimdPath::Avx2Fma;
        }
        if std::arch::is_x86_feature_detected!("sse4.1") {
            return SimdPath::Sse41;
        }
        SimdPath::Scalar
    }
    #[cfg(target_arch = "aarch64")]
    {
        SimdPath::Neon
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        SimdPath::Scalar
    }
}

/// The path DSP kernels will use: `WT_FORCE_SIMD` if set and supported, else the detected one.
///
/// A forced path the CPU cannot run falls back to scalar, never to an unsupported path
/// (that would crash). `WT-DSP-0001` is logged by callers when this lands on scalar.
#[must_use]
pub fn simd_path() -> SimdPath {
    let detected = detected_simd_path();
    let Some(forced) = std::env::var(ENV_FORCE_SIMD)
        .ok()
        .as_deref()
        .and_then(SimdPath::parse)
    else {
        return detected;
    };
    if forced == SimdPath::Scalar || supports(detected, forced) {
        forced
    } else {
        SimdPath::Scalar
    }
}

/// Whether a CPU whose best path is `best` can run `wanted`.
const fn supports(best: SimdPath, wanted: SimdPath) -> bool {
    matches!(
        (best, wanted),
        (SimdPath::Avx2Fma, SimdPath::Avx2Fma | SimdPath::Sse41)
            | (SimdPath::Sse41, SimdPath::Sse41)
            | (SimdPath::Neon, SimdPath::Neon)
            | (_, SimdPath::Scalar)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for p in [
            SimdPath::Avx2Fma,
            SimdPath::Sse41,
            SimdPath::Neon,
            SimdPath::Scalar,
        ] {
            assert_eq!(SimdPath::parse(p.as_str()), Some(p));
        }
    }

    #[test]
    fn unsupported_paths_are_never_selected() {
        assert!(!supports(SimdPath::Neon, SimdPath::Avx2Fma));
        assert!(!supports(SimdPath::Sse41, SimdPath::Avx2Fma));
        assert!(supports(SimdPath::Avx2Fma, SimdPath::Sse41));
        assert!(supports(SimdPath::Neon, SimdPath::Scalar));
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn aarch64_always_has_neon() {
        assert_eq!(detected_simd_path(), SimdPath::Neon);
    }
}
