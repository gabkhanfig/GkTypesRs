#[cfg(windows)]
use winapi;

#[cfg(windows)]
const PF_AVX512F_INSTRUCTIONS_AVAILABLE: u32 = 41;
#[cfg(windows)]
const PF_AVX2_INSTRUCTIONS_AVAILABLE: u32 = 40;

/// Check if AVX-512 is available at runtime.
/// This test will naturally fail if the CPU it's running on doesn't support AVX-512.
/// ```
/// # use gk_types_rs::cpu_features::is_avx512_supported;
/// assert!(is_avx512_supported());
/// ```
#[cfg(windows)]
pub fn is_avx512_supported() -> bool {
    unsafe { 
        return winapi::um::processthreadsapi::IsProcessorFeaturePresent(PF_AVX512F_INSTRUCTIONS_AVAILABLE) == 1
    }
}

/// Check if AVX-2 is available at runtime.
/// This test will naturally fail if the CPU it's running on doesn't support AVX-2.
/// ```
/// # use gk_types_rs::cpu_features::is_avx2_supported;
/// assert!(is_avx2_supported());
/// ```
#[cfg(windows)]
pub fn is_avx2_supported() -> bool {
    unsafe { 
        return winapi::um::processthreadsapi::IsProcessorFeaturePresent(PF_AVX2_INSTRUCTIONS_AVAILABLE) == 1
    }
}

