use std::sync::atomic::{self, AtomicUsize};
use std::ptr::NonNull;
use std::heap::{Alloc, Heap, Layout};
use std::collections::HashSet;
use std::thread;
use std::cmp;
use std::ops::Range;
use std::collections::VecDeque;
use std::fmt;
use std::hash::{Hash, Hasher};

use util::{binomial_coefficient, hash_single, log2_floor};

/// A type of counting network
///
/// See [the module level documentation](index.html) for general information about counting networks.
/// 
/// A bitonic network is constructed recurisvely. A rough pseudocode implementatio would look like
/// ```text
/// fn bitonic(width):
///     upper_wires = bitonic(width / 2)
///     lower_wires = bitonic(width / 2)
///
///     output = merge(upper_wires, lower_wires)
///     return output
/// ```
///
/// The construction of a ``Bitonic[8]`` looks like:
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
/// The base case for ``Bitonic[w]`` is ``Bitonic[1]`` which is a no op, the single wire is 
/// unchanced. The real work of the recursive construction occurs in the ``Merge[w]`` element.
/// The base case of the ``Merge[w]`` network is ``Merge[2]`` which consists of a single balancer.
/// ``Merge[8]`` can be visualized as:
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
/// For the input wires, the even numbered wires (0, 2) of the top inputs to the top 2 slots
/// of the top ``Merge[4]`` network, while the odd numbered wires of (1, 3) of the top 
/// inputs go to top of the 2 slots of the bottom ``Merge[4]`` network. This is flipped for the 
/// bottom 4 inputs, where the odd numbered inputs (5, 7) go to the upper ``Merge[4]`` network,
/// while the evens go to the bottom ``Merge[4]`` network.
pub struct BitonicNetwork<L> {
    // Width of the network
    width: usize,
    // Outputs of the network
    outputs: Vec<NonNull<L>>,
    // Pointers to balancer's memory locations
    balancers: Vec<NonNull<InternalBalancer<L>>>,
}

enum Balancer<L> {
    Internal(NonNull<InternalBalancer<L>>),
    Leaf(NonNull<L>),
}

// Align struct to cache size (Intel)
// This prevents false sharing of the balancer between multiple cores.
#[repr(align(64))]
struct InternalBalancer<L> {
    value: AtomicUsize,
    outputs: [Balancer<L>; 2],
}

impl<L> InternalBalancer<L> {
    fn new() -> Self {
        InternalBalancer {
            value: AtomicUsize::new(0),
            outputs: [
                Balancer::Internal(NonNull::dangling()),
                Balancer::Internal(NonNull::dangling()),
            ],
        }
    }

    fn next<'a>(&'a self) -> &'a Balancer<L> {
        let next_index = self.toggle_up();
        unsafe { self.outputs.get_unchecked(next_index) }
    }

    fn toggle_up(&self) -> usize {
        let old_value = self.value.fetch_add(1, atomic::Ordering::SeqCst);
        old_value % 2
    }
}

impl<L> Balancer<L> {
    fn is_leaf(&self) -> bool {
        match self {
            &Balancer::Internal(_) => false,
            &Balancer::Leaf(_) => true,
        }
    }

    fn leaf_ref(&self) -> &NonNull<L> {
        match self {
            &Balancer::Internal(_) => {
                panic!("called `Balancer::unwrap_leaf()` on a `Internal` value")
            }
            &Balancer::Leaf(ref value) => value,
        }
    }
}

impl<L> BitonicNetwork<L> {
    /// Construct a new network with given width (which must be a power of 2) and outputs.
    ///
    /// Outputs must be ordered corresponding to how they should appear in the network.
    ///
    /// For example in a 4-width network:
    /// ```text
    /// xi = ith input
    /// yi = ith output
    ///
    /// x1 ─────╥────╥─────╥─── y1
    /// x2 ─────╨────║──╥──╨─── y2
    /// x3 ─────╥────║──╨──╥─── y3
    /// x4 ─────╨────╨─────╨─── y4
    /// ```
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
    /// assert_eq!(network.outputs(), vec![&1, &2, &3, &4]);
    /// ```
    pub fn new(outputs: Vec<L>) -> Self {
        assert!(outputs.len().is_power_of_two());

        let width = outputs.len();
        let allocated_outputs = outputs
            .into_iter()
            .map(|output: L| {
                let output_location = Heap.alloc_one::<L>().unwrap();
                unsafe {
                    output_location.as_ptr().write(output);
                }
                output_location
            })
            .collect::<Vec<_>>();

        let num_layers = num_layers(width);
        let layer_width = width / 2;

        let mut wires: Vec<Wire<L>> = construct_bitonic(width, 0);
        debug_assert_eq!(wires.len(), allocated_outputs.len());
        debug_assert_eq!(
            num_layers * layer_width * 2,
            wires.iter().map(|w| w.num_balancers()).sum::<usize>()
        );

        // For each wire, attach the output. This assumes that the outputs are ordered
        // corresponding to the way they should be arranged in the network, e.g.
        // [output for wire 0, output for wire 2, output for wire 3, ...]
        for (wire, output) in wires.iter().zip(allocated_outputs.iter()) {
            let (last_balancer, up) = wire.last();
            unsafe {
                (*last_balancer.as_ptr()).outputs[up as usize] = Balancer::Leaf(output.clone());
            }
        }

        let mut network = BitonicNetwork {
            width,
            outputs: allocated_outputs,
            balancers: Vec::with_capacity(num_layers * layer_width),
        };

        // For each layer in network, fill it with balancers. This method will allow easy
        // access to inputs in the traverse call.
        for _ in 0..num_layers {
            let mut unique_balancers: HashSet<*mut InternalBalancer<L>> = HashSet::new();
            let mut layer: Vec<(usize, NonNull<InternalBalancer<L>>)> = Vec::new();

            for wire in wires.iter_mut() {
                if let Some((balancer, _)) = wire.pop_front() {
                    if !unique_balancers.contains(&balancer.as_ptr()) {
                        unique_balancers.insert(balancer.as_ptr());

                        layer.push((wire.value, balancer));
                    }
                }
            }

            layer.sort_by_key(|&(idx, _)| idx);

            network
                .balancers
                .extend(layer.into_iter().map(|(_, ptr)| ptr));
        }

        network
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
        self.width
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
        let input_slot = hash_single(thread::current().id()) % (self.width as u64);

        let mut current: &Balancer<L> = unsafe {
            self.balancers[get_layer_range(0, self.width / 2)][input_slot as usize / 2]
                .as_ref()
                .next()
        };

        while let &Balancer::Internal(ref balancer) = current {
            current = unsafe { balancer.as_ref().next() };
        }

        assert!(current.is_leaf());
        unsafe { current.leaf_ref().as_ref() }
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
    /// assert_eq!(network.outputs(), vec![&1, &2, &3, &4]);
    /// ```
    pub fn outputs(&self) -> Vec<&L> {
        self.outputs.iter().map(|v| unsafe { v.as_ref() }).collect()
    }
}

impl<L: PartialEq> PartialEq for BitonicNetwork<L> {
    fn eq(&self, other: &Self) -> bool {
        let output_refs = self.outputs.iter().map(|v| unsafe { v.as_ref() });
        let other_outputs = other.outputs.iter().map(|v| unsafe { v.as_ref() });

        output_refs.eq(other_outputs)
    }
}

impl<L: Eq> Eq for BitonicNetwork<L> {}

impl<L: Hash> Hash for BitonicNetwork<L> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.width.hash(state);
        self.outputs
            .iter()
            .map(|v| unsafe { v.as_ref() })
            .for_each(|output| {
                output.hash(state);
            });
    }
}

impl<L: Clone> Clone for BitonicNetwork<L> {
    fn clone(&self) -> Self {
        let outputs: Vec<L> = self.outputs
            .iter()
            .map(|v| unsafe { v.as_ref() })
            .cloned()
            .collect();

        BitonicNetwork::new(outputs)
    }
}

impl<L: fmt::Debug> fmt::Debug for BitonicNetwork<L> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BitonicNetwork")
            .field("width", &self.width)
            .field(
                "outputs",
                &self.outputs
                    .iter()
                    .map(|v| unsafe { v.as_ref() })
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

unsafe impl<L> Send for BitonicNetwork<L> {}
unsafe impl<L> Sync for BitonicNetwork<L> {}

impl<L> Drop for BitonicNetwork<L> {
    fn drop(&mut self) {
        // Drop each internal balancer, leaving NonNull pointers to output
        // Then dealloc balancer memory
        let balancer_layout = Layout::new::<InternalBalancer<L>>();
        for balancer_ptr in self.balancers.iter_mut() {
            unsafe {
                let raw_ptr = balancer_ptr.as_ptr();
                raw_ptr.drop_in_place();
                Heap.dealloc(raw_ptr as *mut u8, balancer_layout.clone());
            }
        }

        // For each output allocation, drop output and dealloc
        let output_layout = Layout::new::<L>();
        for output_ptr in self.outputs.iter_mut() {
            unsafe {
                let raw_ptr = output_ptr.as_ptr();
                raw_ptr.drop_in_place();
                Heap.dealloc(raw_ptr as *mut u8, output_layout.clone());
            }
        }
    }
}

impl<L> From<Vec<L>> for BitonicNetwork<L> {
    fn from(src: Vec<L>) -> Self {
        BitonicNetwork::new(src)
    }
}

fn num_layers(width: usize) -> usize {
    binomial_coefficient((log2_floor(width as u64) + 1) as u64, 2) as usize
}

fn get_layer_range(layer_idx: usize, layer_width: usize) -> Range<usize> {
    let start = layer_width * layer_idx;
    let end = layer_width * (layer_idx + 1);

    start..end
}

struct Wire<L> {
    balancer_history: VecDeque<(NonNull<InternalBalancer<L>>, bool)>,
    value: usize,
}

impl<L> PartialEq for Wire<L> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<L> PartialOrd for Wire<L> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<L> Eq for Wire<L> {}

impl<L> Ord for Wire<L> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<L> Wire<L> {
    fn num_balancers(&self) -> usize {
        self.balancer_history.len()
    }

    fn last(&self) -> (NonNull<InternalBalancer<L>>, bool) {
        self.balancer_history[self.balancer_history.len() - 1]
    }

    fn add(&mut self, balancer: NonNull<InternalBalancer<L>>, up: bool) {
        self.balancer_history.push_back((balancer, up));
    }

    fn pop_front(&mut self) -> Option<(NonNull<InternalBalancer<L>>, bool)> {
        self.balancer_history.pop_front()
    }
}

fn split_even_odd<L>(wires: Vec<Wire<L>>) -> (Vec<Wire<L>>, Vec<Wire<L>>) {
    let (even_wires, odd_wires): (Vec<(usize, Wire<L>)>, Vec<(usize, Wire<L>)>) = wires
        .into_iter()
        .enumerate()
        .partition(|&(idx, _)| idx % 2 == 0);

    let even = even_wires
        .into_iter()
        .map(|(_, value)| value)
        .collect::<Vec<_>>();

    let odd = odd_wires
        .into_iter()
        .map(|(_, value)| value)
        .collect::<Vec<_>>();

    (even, odd)
}

fn merge_wires<L>(upper: Vec<Wire<L>>, lower: Vec<Wire<L>>) -> Vec<Wire<L>> {
    let pairs = upper.into_iter().zip(lower.into_iter());

    let mut wires = Vec::new();
    for (mut upper_wire, mut lower_wire) in pairs {
        debug_assert_eq!(upper_wire.num_balancers(), lower_wire.num_balancers());

        let new_balancer = InternalBalancer::new();
        let new_balancer_alloc = Heap.alloc_one::<InternalBalancer<L>>().unwrap();
        unsafe {
            new_balancer_alloc.as_ptr().write(new_balancer);

            if upper_wire.num_balancers() > 0 {
                let (last, up) = upper_wire.last();
                let mut temp = last.as_ptr().read();
                temp.outputs[up as usize] = Balancer::Internal(new_balancer_alloc);
                last.as_ptr().write(temp);
            }

            if lower_wire.num_balancers() > 0 {
                let (mut last, up) = lower_wire.last();
                let mut temp = last.as_ptr().read();
                temp.outputs[up as usize] = Balancer::Internal(new_balancer_alloc);
                last.as_ptr().write(temp);
            }
        }

        upper_wire.add(new_balancer_alloc, false);
        lower_wire.add(new_balancer_alloc, true);

        wires.push(upper_wire);
        wires.push(lower_wire);
    }

    wires
}

fn construct_bitonic<L>(width: usize, wire_index: usize) -> Vec<Wire<L>> {
    if width == 1 {
        vec![
            Wire {
                balancer_history: VecDeque::new(),
                value: wire_index,
            },
        ]
    } else {
        let upper_result = construct_bitonic(width / 2, wire_index);
        let lower_result = construct_bitonic(width / 2, wire_index + width / 2);

        merge_networks(upper_result, lower_result)
    }
}

fn merge_networks<L>(upper: Vec<Wire<L>>, lower: Vec<Wire<L>>) -> Vec<Wire<L>> {
    if upper.len() + lower.len() == 2 {
        merge_wires(upper, lower)
    } else {
        let (upper_even, upper_odd) = split_even_odd(upper);
        let (lower_even, lower_odd) = split_even_odd(lower);

        let upper_result = merge_networks(upper_even, lower_odd);
        let lower_result = merge_networks(upper_odd, lower_even);

        merge_wires(upper_result, lower_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
