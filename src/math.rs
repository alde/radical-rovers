//! Small numeric helpers shared by terrain and gameplay code.

pub fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Deterministic value in [0, 1) for an integer grid cell, so procedural
/// detail is identical on every visit.
pub fn hash2(i: i64, j: i64, salt: u32) -> f64 {
    let mut h = (i as u32)
        .wrapping_mul(374_761_393)
        .wrapping_add((j as u32).wrapping_mul(668_265_263))
        .wrapping_add(salt.wrapping_mul(2_246_822_519));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    f64::from(h) / 4_294_967_296.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic_and_in_range() {
        let a = hash2(12, -7, 3);
        assert_eq!(a, hash2(12, -7, 3));
        assert!((0.0..1.0).contains(&a));
        assert_ne!(a, hash2(13, -7, 3));
    }

    #[test]
    fn smoothstep_clamps_and_eases() {
        assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
        assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
    }
}
