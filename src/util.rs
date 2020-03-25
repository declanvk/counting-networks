use core::{
    hash::{Hash, Hasher},
    ops::Range,
};
use std::collections::hash_map::DefaultHasher;

pub fn hash_single<T>(value: T) -> u64
where
    T: Hash,
{
    let mut hasher = DefaultHasher::new();

    value.hash(&mut hasher);

    hasher.finish()
}

// TODO: remove and use `slice.as_ptr_range` when it becomes stable.
pub fn slice_to_ptr_range<T>(slice: &[T]) -> Range<*const T> {
    // The `add` here is safe, because:
    //
    //   - Both pointers are part of the same object, as pointing directly past the
    //     object also counts.
    //
    //   - The size of the slice is never larger than isize::MAX bytes, as noted
    //     here:
    //       - https://github.com/rust-lang/unsafe-code-guidelines/issues/102#issuecomment-473340447
    //       - https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    //       - https://doc.rust-lang.org/core/slice/fn.from_raw_parts.html#safety
    //     (This doesn't seem normative yet, but the very same assumption is
    //     made in many places, including the Index implementation of slices.)
    //
    //   - There is no wrapping around involved, as slices do not wrap past the end
    //     of the address space.
    //
    // See the documentation of pointer::add.
    let start = slice.as_ptr();
    let end = unsafe { start.add(slice.len()) };
    start..end
}

// TODO: remove #[allow(dead_code)]

#[allow(dead_code)]
pub const E_COCHAIN: [usize; 1] = [0b0];
#[allow(dead_code)]
pub const O_COCHAIN: [usize; 1] = [0b1];
#[allow(dead_code)]
pub const A_COCHAIN: [usize; 2] = [0b00, 0b11];
#[allow(dead_code)]
pub const B_COCHAIN: [usize; 2] = [0b01, 0b10];

#[allow(dead_code)]
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
        assert_eq!(chain_1, &[0, 3, 4, 7, 8, 11, 12, 15, 16, 19]);

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
