use super::common::{Network, NetworkConfiguration};
use core::{iter::FusedIterator, ops::Range};
use std::vec;

/// A type of counting network
///
/// See [the module level documentation](index.html) for general information
/// about counting networks.
///
/// A bitonic network is constructed recursively. A rough pseudo-code
/// implementation would look like
///
/// ```text
/// fn bitonic(width):
///  upper_wires = bitonic(width / 2)
///  lower_wires = bitonic(width / 2)
///
///  output = merge(upper_wires, lower_wires)
///  return output
/// ```
///
/// The construction of a `Bitonic[8]` looks like:
/// ```text
///      ┌────────────────┐          ┌──────────────┐
/// ─────┤                ├──────────┤              ├──────────
/// ─────┤   Bitonic[4]   ├──────────┤              ├──────────
/// ─────┤                ├──────────┤              ├──────────
/// ─────┤                ├──────────┤              ├──────────
///      └────────────────┘          │   Merge[8]   │
///      ┌────────────────┐          │              │
/// ─────┤                ├──────────┤              ├──────────
/// ─────┤   Bitonic[4]   ├──────────┤              ├──────────
/// ─────┤                ├──────────┤              ├──────────
/// ─────┤                ├──────────┤              ├──────────
///      └────────────────┘          └──────────────┘
/// ```
///
/// The base case for `Bitonic[w]` is `Bitonic[1]` which is a no op, the
/// single wire is unchanged. The real work of the recursive construction occurs
/// in the `Merge[w]` element. The base case of the `Merge[w]` network is
/// `Merge[2]` which consists of a single balancer. `Merge[8]` can be
/// visualized as:
///
/// ```text
///                           ┌────────────────┐
/// x0 ───────────────────────┤                ├─────────────┲┱── y0
/// x1 ─────┐ ┌───────────────┤    Merge[4]    ├────┐  ┌─────┺┹── y1
/// x2 ─────┼─┘ ┌─────────────┤                ├───┐└──┼─────┲┱── y2
/// x3 ───┐ │   │ ┌───────────┤                ├─┐ │ ┌─┼─────┺┹── y3
///       │ └───┼─┼───────┐   └────────────────┘ └─┼─┼─┼─┐
///       └─────┼─┼─────┐ │   ┌────────────────┐   └─┼─┼─┼─┐
/// x4 ─────────┼─┼───┐ │ └───┤                ├─────┼─┘ │ └─┲┱── y4
/// x5 ─────────┘ │   │ └─────┤    Merge[4]    ├─────┘┌──┼───┺┹── y5
/// x6 ───────────┼─┐ └───────┤                ├──────┘  └───┲┱── y6
/// x7 ───────────┘ └─────────┤                ├─────────────┺┹── y7
///                           └────────────────┘
/// ┏┓
/// ┗┛ are balancers, xi is the ith wire, yi is the ith output
/// ```
///
/// For the input wires, the even numbered wires (0, 2) of the top inputs to the
/// top 2 slots of the top `Merge[4]` network, while the odd numbered wires of
/// (1, 3) of the top inputs go to top of the 2 slots of the bottom `Merge[4]`
/// network. This is flipped for the bottom 4 inputs, where the odd numbered
/// inputs (5, 7) go to the upper `Merge[4]` network, while the evens go to
/// the bottom `Merge[4]` network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitonicNetwork<L>(Network<L, BitonicConfiguration>);

impl<L> BitonicNetwork<L> {
    /// Construct a new network with given width (which must be a power of 2)
    /// and outputs.
    ///
    /// Outputs must be ordered corresponding to how they should appear in the
    /// network.
    ///
    /// For example in a 4-width network:
    ///
    /// ```text
    /// xi = ith input
    /// yi = ith output
    ///
    /// x1 ─────╥────╥─────╥─── y1
    /// x2 ─────╨────║──╥──╨─── y2
    /// x3 ─────╥────║──╨──╥─── y3
    /// x4 ─────╨────╨─────╨─── y4
    /// ```
    ///
    /// The outputs passed through should appear [y1, y2, y3, y4]
    ///
    /// # Examples
    ///
    /// ```
    /// use counting_networks::networks::BitonicNetwork;
    ///
    /// let outputs = vec![1, 2, 3, 4];
    ///
    /// let network = BitonicNetwork::new(outputs);
    ///
    /// assert_eq!(network.width(), 4);
    /// assert_eq!(network.outputs(), &[1, 2, 3, 4]);
    /// ```
    pub fn new(outputs: Vec<L>) -> Self {
        assert!(outputs.len().is_power_of_two());

        BitonicNetwork(Network::new(outputs))
    }

    /// Returns the width of the network.
    ///
    /// # Examples
    ///
    /// ```
    /// use counting_networks::networks::BitonicNetwork;
    ///
    /// let network = BitonicNetwork::new(vec![1, 2, 3, 4]);
    ///
    /// assert_eq!(network.width(), 4);
    /// ```
    pub fn width(&self) -> usize {
        self.0.width()
    }

    /// Traverse the network and obtain a reference to an output element.
    ///
    /// # Examples
    ///
    /// ```
    /// use counting_networks::networks::BitonicNetwork;
    ///
    /// let network = BitonicNetwork::new(vec![1, 2, 3, 4]);
    ///
    /// assert_eq!(network.traverse(), &1);
    /// assert_eq!(network.traverse(), &2);
    /// assert_eq!(network.traverse(), &3);
    /// assert_eq!(network.traverse(), &4);
    /// ```
    pub fn traverse(&self) -> &L {
        self.0.traverse()
    }

    /// Get references to all the outputs of the network.
    ///
    /// # Examples
    ///
    /// ```
    /// use counting_networks::networks::BitonicNetwork;
    ///
    /// let network = BitonicNetwork::new(vec![1, 2, 3, 4]);
    ///
    /// assert_eq!(network.outputs(), &[1, 2, 3, 4]);
    /// ```
    pub fn outputs(&self) -> &[L] {
        self.0.outputs()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BitonicConfiguration(usize);

impl IntoIterator for BitonicConfiguration {
    type IntoIter = BitonicConfigurationIter;
    type Item = (usize, usize);

    fn into_iter(self) -> Self::IntoIter {
        BitonicConfigurationIter {
            stack: vec![BitonicStep::Split(0..self.0)],
            output_stack: vec![],
        }
    }
}

impl NetworkConfiguration for BitonicConfiguration {
    fn from_width(width: usize) -> Self {
        BitonicConfiguration(width)
    }
}

#[derive(Debug, Clone)]
pub struct BitonicConfigurationIter {
    stack: Vec<BitonicStep>,
    output_stack: Vec<usize>,
}

impl FusedIterator for BitonicConfigurationIter {}

#[derive(Debug, Clone)]
enum BitonicStep {
    Split(Range<usize>),
    Merge(usize),
    Output((usize, usize)),
}

impl BitonicConfigurationIter {
    fn split(&mut self, wires: Range<usize>) {
        let width = wires.clone().count();
        if wires.clone().count() > 1 {
            self.output_stack.extend(wires.clone());
            // Push a merge with the expected number of wires
            self.stack.push(BitonicStep::Merge(width));

            let middle = (wires.end - wires.start) / 2 + wires.start;
            // Continue splitting with half of wires going towards each
            self.stack.push(BitonicStep::Split(middle..wires.end));
            // This half will go first
            self.stack.push(BitonicStep::Split(wires.start..middle));
        }
    }

    fn merge(&mut self, width: usize) {
        let merge_range = (self.output_stack.len() - width)..self.output_stack.len();
        let to_merge = &self.output_stack[merge_range.clone()];

        let pair_iter = to_merge
            .iter()
            .step_by(2)
            .cloned()
            .zip(to_merge.iter().skip(1).step_by(2).cloned());

        self.stack.extend(
            pair_iter
                .map(BitonicStep::Output)
                // important that it is reversed here
                .rev(),
        );

        if width > 2 {
            let full_range = (self.output_stack.len() - width)..self.output_stack.len();
            let middle = (full_range.end - full_range.start) / 2 + full_range.start;
            let top_range = full_range.start..middle;
            let bottom_range = middle..full_range.end;

            let top_even: Vec<_> = top_range
                .clone()
                .step_by(2)
                .map(|idx| self.output_stack[idx]);
            let top_odd: Vec<_> = top_range
                .skip(1)
                .step_by(2)
                .map(|idx| self.output_stack[idx]);

            let bottom_even: Vec<_> = bottom_range
                .clone()
                .step_by(2)
                .map(|idx| self.output_stack[idx]);
            let bottom_odd: Vec<_> = bottom_range
                .skip(1)
                .step_by(2)
                .map(|idx| self.output_stack[idx]);

            // The bottom merge goes into the stack first
            self.output_stack
                .extend(top_odd.chain(bottom_even));
            self.stack.push(BitonicStep::Merge(width / 2));

            // The top merge will be processed first
            self.output_stack
                .extend(top_even.chain(bottom_odd));
            self.stack.push(BitonicStep::Merge(width / 2));
        }

        self.output_stack.drain(merge_range);
    }
}

impl Iterator for BitonicConfigurationIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(next_step) = self.stack.pop() {
            match next_step {
                BitonicStep::Split(wires) => self.split(wires),
                BitonicStep::Merge(width) => self.merge(width),
                BitonicStep::Output(balancer) => {
                    return Some(balancer);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Split[4]: (0..4)
    ///      - Split[2]: (0..2)
    ///          - Split[1]: (0..1)
    ///          - Split[1]: (1..2)
    ///          - Merge[2]: [0, 1]
    ///              - Output (0, 1)
    ///      - Split[2]: (2..4)
    ///          - Split[1]: (2..3)
    ///          - Split[1]: (3..4)
    ///          - Merge[2]: [2, 3]
    ///              - Output (2, 3)
    ///      - Merge[4]: [0, 1, 2, 3]
    ///          - Merge[2]: [0, 3]
    ///              - Output (0, 3)
    ///          - Merge[2]: [1, 2]
    ///              - Output (1, 2)
    ///          - Output (0, 1)
    ///          - Output (2, 3)
    #[test]
    fn bitonic_4_configuration() {
        let config = BitonicConfiguration(4);

        let balancers: Vec<_> = config.into_iter().collect();

        assert_eq!(
            &balancers,
            &[(0, 1), (2, 3), (0, 3), (1, 2), (0, 1), (2, 3)]
        )
    }

    /// Split[8]: (0..8)
    ///  - Split[4]: (0..4)
    ///      - Split[2]: (0..2)
    ///          - Split[1]: (0..1)
    ///          - Split[1]: (1..2)
    ///          - Merge[2]: [0, 1]
    ///              - Output (0, 1)
    ///      - Split[2]: (2..4)
    ///          - Split[1]: (2..3)
    ///          - Split[1]: (3..4)
    ///          - Merge[2]: [2, 3]
    ///              - Output (2, 3)
    ///      - Merge[4]: [0, 1, 2, 3]
    ///          - Merge[2]: [0, 3]
    ///              - Output (0, 3)
    ///          - Merge[2]: [1, 2]
    ///              - Output (1, 2)
    ///          - Output (0, 1)
    ///          - Output (2, 3)
    ///  - Split[4]: (4..8)
    ///      - Split[2]: (4..6)
    ///          - Split[1]: (4..5)
    ///          - Split[1]: (5..6)
    ///          - Merge[2]: [4, 5]
    ///              - Output (4, 5)
    ///      - Split[2]: (6..8)
    ///          - Split[1]: (6..7)
    ///          - Split[1]: (7..8)
    ///          - Merge[2]: [6, 7]
    ///              - Output (6, 7)
    ///      - Merge[4]: [4, 5, 6, 7]
    ///          - Merge[2]: [4, 7]
    ///              - Output (4, 7)
    ///          - Merge[2]: [5, 6]
    ///              - Output (5, 6)
    ///          - Output (4, 5)
    ///          - Output (6, 7)
    ///  - Merge[8]: [0, 1, 2, 3, 4, 5, 6, 7]
    ///      - Merge[4]: [0, 2, 5, 7]
    ///          - Merge[2]: [0, 7]
    ///              - Output (0, 7)
    ///          - Merge[2]: [2, 5]
    ///              - Output (2, 5)
    ///          - Output (0, 2)
    ///          - Output (5, 7)
    ///      - Merge[4]: [1, 3, 4, 6]
    ///          - Merge[2]: [1, 6]
    ///              - Output (1, 6)
    ///          - Merge[2]: [3, 4]
    ///              - Output (3, 4)
    ///          - Output (1, 3)
    ///          - Output (4, 6)
    ///      - Output (0, 1)
    ///      - Output (2, 3)
    ///      - Output (4, 5)
    ///      - Output (6, 7)
    #[test]
    fn bitonic_8_configuration() {
        let config = BitonicConfiguration(8);

        let balancers: Vec<_> = config.into_iter().collect();

        assert_eq!(
            &balancers,
            &[
                (0, 1),
                (2, 3),
                (0, 3),
                (1, 2),
                (0, 1),
                (2, 3),
                (4, 5),
                (6, 7),
                (4, 7),
                (5, 6),
                (4, 5),
                (6, 7),
                (0, 7),
                (2, 5),
                (0, 2),
                (5, 7),
                (1, 6),
                (3, 4),
                (1, 3),
                (4, 6),
                (0, 1),
                (2, 3),
                (4, 5),
                (6, 7),
            ]
        )
    }

    #[test]
    fn is_send() {
        fn send_only<T: Send>(_: T) {}

        send_only(BitonicNetwork::new(vec![1; 4]));
    }

    #[test]
    fn is_sync() {
        fn sync_only<T: Sync>(_: T) {}

        sync_only(BitonicNetwork::new(vec![1; 4]));
    }

    #[test]
    fn initialize_network() {
        const WIDTH: usize = 16;

        let network = BitonicNetwork::new(vec![1; WIDTH]);

        assert_eq!(network.width(), WIDTH);
    }

    #[test]
    #[should_panic]
    fn initialize_network_bad_width() {
        let _ = BitonicNetwork::new(vec![1, 2, 3]);
    }

    #[test]
    fn traverse_network() {
        const WIDTH: usize = 16;
        let outputs = (1..(WIDTH + 1)).collect::<Vec<_>>();
        let network = BitonicNetwork::new(outputs);

        for output in 1..(WIDTH + 1) {
            assert_eq!(network.traverse(), &output);
        }
    }
}
