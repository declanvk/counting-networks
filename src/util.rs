use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

pub fn log2_floor(n: u64) -> u32 {
    63 - n.leading_zeros()
}

pub fn binomial_coefficient(n: u64, mut k: u64) -> u64 {
    if k > n - k {
        k = n - k;
    }

    // Calculate value of [n * (n-1) *---* (n-k+1)] / [k * (k-1) *----* 1]
    let mut result: u64 = 1;
    for value in 1..(k + 1) {
        result *= n + 1 - value;
        result /= value;
    }

    result
}

pub fn hash_single<T>(value: T) -> u64 where T: Hash {
    let mut hasher = DefaultHasher::new();

    value.hash(&mut hasher);

    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_binomial_low_edge_values() {
        assert_eq!(binomial_coefficient(0, 0), 1);

        assert_eq!(binomial_coefficient(1, 0), 1);
        assert_eq!(binomial_coefficient(1, 1), 1);

        assert_eq!(binomial_coefficient(2, 0), 1);
        assert_eq!(binomial_coefficient(2, 1), 2);
        assert_eq!(binomial_coefficient(2, 2), 1);

        assert_eq!(binomial_coefficient(3, 0), 1);
        assert_eq!(binomial_coefficient(3, 1), 3);
        assert_eq!(binomial_coefficient(3, 2), 3);
        assert_eq!(binomial_coefficient(3, 3), 1);

        assert_eq!(binomial_coefficient(4, 0), 1);
        assert_eq!(binomial_coefficient(4, 1), 4);
        assert_eq!(binomial_coefficient(4, 2), 6);
        assert_eq!(binomial_coefficient(4, 3), 4);
        assert_eq!(binomial_coefficient(4, 4), 1);

        assert_eq!(binomial_coefficient(5, 0), 1);
        assert_eq!(binomial_coefficient(5, 1), 5);
        assert_eq!(binomial_coefficient(5, 2), 10);
        assert_eq!(binomial_coefficient(5, 3), 10);
        assert_eq!(binomial_coefficient(5, 4), 5);
        assert_eq!(binomial_coefficient(5, 5), 1);

        assert_eq!(binomial_coefficient(6, 0), 1);
        assert_eq!(binomial_coefficient(6, 1), 6);
        assert_eq!(binomial_coefficient(6, 2), 15);
        assert_eq!(binomial_coefficient(6, 3), 20);
        assert_eq!(binomial_coefficient(6, 4), 15);
        assert_eq!(binomial_coefficient(6, 5), 6);
        assert_eq!(binomial_coefficient(6, 6), 1);

        assert_eq!(binomial_coefficient(7, 0), 1);
        assert_eq!(binomial_coefficient(7, 1), 7);
        assert_eq!(binomial_coefficient(7, 2), 21);
        assert_eq!(binomial_coefficient(7, 3), 35);
        assert_eq!(binomial_coefficient(7, 4), 35);
        assert_eq!(binomial_coefficient(7, 5), 21);
        assert_eq!(binomial_coefficient(7, 6), 7);
        assert_eq!(binomial_coefficient(7, 7), 1);
    }

    #[test]
    fn log2_floor_correct_powers_of_2() {
        assert_eq!(log2_floor(1), 0);
        assert_eq!(log2_floor(2), 1);
        assert_eq!(log2_floor(4), 2);
        assert_eq!(log2_floor(8), 3);
        assert_eq!(log2_floor(16), 4);
        assert_eq!(log2_floor(32), 5);
        assert_eq!(log2_floor(64), 6);
        assert_eq!(log2_floor(128), 7);
        assert_eq!(log2_floor(256), 8);
        assert_eq!(log2_floor(512), 9);
        assert_eq!(log2_floor(1024), 10);
        assert_eq!(log2_floor(2048), 11);
        assert_eq!(log2_floor(4096), 12);
        assert_eq!(log2_floor(8192), 13);
        assert_eq!(log2_floor(16384), 14);
        assert_eq!(log2_floor(32768), 15);
        assert_eq!(log2_floor(65536), 16);
    }
}