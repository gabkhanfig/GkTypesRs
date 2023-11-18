use std::arch::x86_64::{__m512i, __m256i};

pub(crate) fn simd_find_epi8_512(buffer: *const i8, length: usize, capacity: usize, element: i8) -> Option<usize> {
    unsafe {
        const NUM_PER_SIMD: usize = 64;
        let mut i: usize = 0;
        let element_vec = std::arch::x86_64::_mm512_set1_epi8(element);
        for _ in (0..capacity).step_by(NUM_PER_SIMD) {
            let this_vec = buffer.offset(i as isize) as *const __m512i;
            let mask = std::arch::x86_64::_mm512_cmpeq_epi8_mask(*this_vec, element_vec);
            if mask != 0 {
                let lowest = mask.trailing_zeros() as usize;
                if lowest + length <= capacity {
                    return Some(lowest + i);
                }
            }
            i += NUM_PER_SIMD;
        }
        None
    } 
}

pub(crate) fn simd_find_epi8_256(buffer: *const i8, length: usize, capacity: usize, element: i8) -> Option<usize> {
    unsafe {
        const NUM_PER_SIMD: usize = 32;
        let mut i: usize = 0;
        let element_vec = std::arch::x86_64::_mm256_set1_epi8(element);
        for _ in (0..capacity).step_by(NUM_PER_SIMD) {
            let this_vec = buffer.offset(i as isize) as *const __m256i;
            let mask = std::arch::x86_64::_mm256_cmpeq_epi8_mask(*this_vec, element_vec);
            if mask != 0 {
                let lowest = mask.trailing_zeros() as usize;
                if lowest + length <= capacity {
                    return Some(lowest + i);
                }
            }
            i += NUM_PER_SIMD;
        }
        None
    } 
}

pub(crate) fn simd_find_epi16_512(buffer: *const i16, length: usize, capacity: usize, element: i16) -> Option<usize> {
    unsafe {
        const NUM_PER_SIMD: usize = 32;
        let mut i: usize = 0;
        let element_vec = std::arch::x86_64::_mm512_set1_epi16(element);
        for _ in (0..capacity).step_by(NUM_PER_SIMD) {
            let this_vec = buffer.offset(i as isize) as *const __m512i;
            let mask = std::arch::x86_64::_mm512_cmpeq_epi16_mask(*this_vec, element_vec);
            if mask != 0 {
                let lowest = mask.trailing_zeros() as usize;
                if lowest + length <= capacity {
                    return Some(lowest + i);
                }
            }
            i += NUM_PER_SIMD;
        }
        None
    } 
}

pub(crate) fn simd_find_epi16_256(buffer: *const i16, length: usize, capacity: usize, element: i16) -> Option<usize> {
    unsafe {
        const NUM_PER_SIMD: usize = 16;
        let mut i: usize = 0;
        let element_vec = std::arch::x86_64::_mm256_set1_epi16(element);
        for _ in (0..capacity).step_by(NUM_PER_SIMD) {
            let this_vec = buffer.offset(i as isize) as *const __m256i;
            let mask = std::arch::x86_64::_mm256_cmpeq_epi16_mask(*this_vec, element_vec);
            if mask != 0 {
                let lowest = mask.trailing_zeros() as usize;
                if lowest + length <= capacity {
                    return Some(lowest + i);
                }
            }
            i += NUM_PER_SIMD;
        }
        None
    } 
}

pub(crate) fn simd_find_epi32_512(buffer: *const i32, length: usize, capacity: usize, element: i32) -> Option<usize> {
    unsafe {
        const NUM_PER_SIMD: usize = 16;
        let mut i: usize = 0;
        let element_vec = std::arch::x86_64::_mm512_set1_epi32(element);
        for _ in (0..capacity).step_by(NUM_PER_SIMD) {
            let this_vec = buffer.offset(i as isize) as *const __m512i;
            let mask = std::arch::x86_64::_mm512_cmpeq_epi32_mask(*this_vec, element_vec);
            if mask != 0 {
                let lowest = mask.trailing_zeros() as usize;
                if lowest + length <= capacity {
                    return Some(lowest + i);
                }
            }
            i += NUM_PER_SIMD;
        }
        None
    } 
}

pub(crate) fn simd_find_epi32_256(buffer: *const i32, length: usize, capacity: usize, element: i32) -> Option<usize> {
    unsafe {
        const NUM_PER_SIMD: usize = 8;
        let mut i: usize = 0;
        let element_vec = std::arch::x86_64::_mm256_set1_epi32(element);
        for _ in (0..capacity).step_by(NUM_PER_SIMD) {
            let this_vec = buffer.offset(i as isize) as *const __m256i;
            let mask = std::arch::x86_64::_mm256_cmpeq_epi32_mask(*this_vec, element_vec);
            if mask != 0 {
                let lowest = mask.trailing_zeros() as usize;
                if lowest + length <= capacity {
                    return Some(lowest + i);
                }
            }
            i += NUM_PER_SIMD;
        }
        None
    } 
}

pub(crate) fn simd_find_epi64_512(buffer: *const i64, length: usize, capacity: usize, element: i64) -> Option<usize> {
    unsafe {
        const NUM_PER_SIMD: usize = 8;
        let mut i: usize = 0;
        let element_vec = std::arch::x86_64::_mm512_set1_epi64(element);
        for _ in (0..capacity).step_by(NUM_PER_SIMD) {
            let this_vec = buffer.offset(i as isize) as *const __m512i;
            let mask = std::arch::x86_64::_mm512_cmpeq_epi64_mask(*this_vec, element_vec);
            if mask != 0 {
                let lowest = mask.trailing_zeros() as usize;
                if lowest + length <= capacity {
                    return Some(lowest + i);
                }
            }
            i += NUM_PER_SIMD;
        }
        None
    }
}

pub(crate) fn simd_find_epi64_256(buffer: *const i64, length: usize, capacity: usize, element: i64) -> Option<usize> {
    unsafe {
        const NUM_PER_SIMD: usize = 4;
        let mut i: usize = 0;
        let element_vec = std::arch::x86_64::_mm256_set1_epi64x(element);
        for _ in (0..capacity).step_by(NUM_PER_SIMD) {
            let this_vec = buffer.offset(i as isize) as *const __m256i;
            let mask = std::arch::x86_64::_mm256_cmpeq_epi64_mask(*this_vec, element_vec);
            if mask != 0 {
                let lowest = mask.trailing_zeros() as usize;
                if lowest + length <= capacity {
                    return Some(lowest + i);
                }
            }
            i += NUM_PER_SIMD;
        }
        None
    }
}