use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::ops::Range;

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

pub const E_COCHAIN: [usize; 1] = [0b0];
pub const O_COCHAIN: [usize; 1] = [0b1];
pub const A_COCHAIN: [usize; 2] = [0b00, 0b11];
pub const B_COCHAIN: [usize; 2] = [0b01, 0b10];

pub fn generate_cochain(range: Range<usize>, prefixes: &[usize]) -> Vec<usize> {
    let mask = (1 << prefixes.len()) - 1;

    let mut output = Vec::new();

    for idx in range {
        for &prefix in prefixes {
            if (idx & mask) == prefix {
                output.push(idx);
            }
        }
    }

    output
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

    #[test]
    fn check_even_cochains() {
        let chain_1 = generate_cochain(0..20, &E_COCHAIN);
        assert!(chain_1.iter().all(|&x| x % 2 == 0));
        assert_eq!(chain_1.len(), 10);

        let chain_2 = generate_cochain(0..1, &E_COCHAIN);
        assert!(chain_2.iter().all(|&x| x % 2 == 0));
        assert_eq!(chain_2.len(), 1);

        let chain_3 = generate_cochain(0..13, &E_COCHAIN);
        assert!(chain_3.iter().all(|&x| x % 2 == 0));
        assert_eq!(chain_3.len(), 7);
    }

    #[test]
    fn check_odd_cochain() {
        let chain_1 = generate_cochain(0..20, &O_COCHAIN);
        assert!(chain_1.iter().all(|&x| x % 2 != 0));
        assert_eq!(chain_1.len(), 10);

        let chain_2 = generate_cochain(0..1, &O_COCHAIN);
        assert!(chain_2.iter().all(|&x| x % 2 != 0));
        assert_eq!(chain_2.len(), 0);

        let chain_3 = generate_cochain(0..13, &O_COCHAIN);
        assert!(chain_3.iter().all(|&x| x % 2 != 0));
        assert_eq!(chain_3.len(), 6);
    }

    #[test]
    #[allow(non_snake_case)]
    fn check_A_cochain() {
        let chain_1 = generate_cochain(0..20, &A_COCHAIN);
        assert_eq!(chain_1, &[0, 3, 4,7, 8, 11, 12, 15, 16, 19]);

        let chain_1 = generate_cochain(0..1, &A_COCHAIN);
        assert_eq!(chain_1, &[0]);

        let chain_1 = generate_cochain(0..14, &A_COCHAIN);
        assert_eq!(chain_1, &[0, 3, 4, 7, 8, 11, 12]);
    }

    #[test]
    #[allow(non_snake_case)]
    fn check_B_cochain() {
        let chain_1 = generate_cochain(0..20, &B_COCHAIN);
        assert_eq!(chain_1, &[1, 2, 5, 6, 9, 10, 13, 14, 17, 18]);

        let chain_1 = generate_cochain(0..1, &B_COCHAIN);
        assert_eq!(chain_1, &[]);

        let chain_1 = generate_cochain(0..14, &B_COCHAIN);
        assert_eq!(chain_1, &[1, 2, 5, 6, 9, 10, 13]);
    }
}