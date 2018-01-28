use std::sync::atomic::{self, AtomicUsize};
use std::ptr::NonNull;
use std::heap::{Alloc, Heap, Layout};
use std::collections::HashSet;
use std::thread;
use std::slice;
use std::cmp;

use alloc::raw_vec::RawVec;

use util::{binomial_coefficient, log2_floor, hash_single};

// Needs custom drop logic to ensure balancers are cleaned up
pub struct BitonicNetwork<L> {
    // Width of the network
    width: usize,
    // Number of layers in the network
    num_layers: usize,
    // Outputs of the network
    outputs: Vec<NonNull<L>>,
    // Pointers to balancer's memory locations
    balancers: RawVec<NonNull<InternalBalancer<L>>>,
}

// Can be dropped without custom logic
enum Balancer<L> {
    Internal(NonNull<InternalBalancer<L>>),
    Leaf(NonNull<L>),
}

// Can be dropped without custom logic
struct InternalBalancer<L> {
    value: AtomicUsize,
    outputs: [Balancer<L>; 2],
}

impl<L> InternalBalancer<L> {
    fn new() -> Self {
        InternalBalancer {
            value: AtomicUsize::new(0),
            outputs: [Balancer::Internal(NonNull::dangling()), Balancer::Internal(NonNull::dangling())]
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

    fn is_internal(&self) -> bool {
        !self.is_leaf()
    }

    fn leaf_ref(&self) -> &NonNull<L> {
        match self {
            &Balancer::Internal(_) => {
                panic!("called `Balancer::unwrap_leaf()` on a `Internal` value")
            }
            &Balancer::Leaf(ref value) => value,
        }
    }

    fn unwrap_leaf(self) -> NonNull<L> {
        match self {
            Balancer::Internal(_) => {
                panic!("called `Balancer::unwrap_leaf()` on a `Internal` value")
            }
            Balancer::Leaf(value) => value,
        }
    }

    fn unwrap_internal(self) -> NonNull<InternalBalancer<L>> {
        match self {
            Balancer::Internal(balancer) => balancer,
            Balancer::Leaf(value) => {
                panic!("called `Balancer::unwrap_internal()` on a `Leaf` value")
            }
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
    pub fn new(width: usize, outputs: Vec<L>) -> Self {
        assert!(width.is_power_of_two());
        assert_eq!(width, outputs.len());

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

        let num_layers = BitonicNetwork::<L>::num_layers(width);
        let layer_width = width / 2;

        let mut wires: Vec<Wire<L>> = construct_bitonic(width, 0);
        debug_assert_eq!(wires.len(), allocated_outputs.len());
        debug_assert_eq!(num_layers * layer_width, wires.iter().map(|w| w.num_balancers()).sum::<usize>());

        // Based on Ord implementation, will sort wires by value (1, 2, 3, ...)
        wires.sort();

        // For each wire, attach the output. This assumes that the outputs are ordered
        // corresponding to the way they should be arranged in the network, e.g.
        // [output for wire 0, output for wire 2, output for wire 3, ...]
        for (wire, output) in wires.iter().zip(allocated_outputs.iter()) {
            let (last_balancer, up) = wire.last();
            unsafe { (*last_balancer.as_ptr()).outputs[up as usize] = Balancer::Leaf(output.clone()); }
        }

        // let unique_raw_ptrs: HashSet<*mut InternalBalancer<L>> = wires.into_iter().flat_map(|w| w.balancer_history.into_iter()).map(|b| b.0.as_ptr()).collect();

        // unique_raw_ptrs.into_iter().filter_map(|ptr| NonNull::new(ptr))

        let mut network = BitonicNetwork {
            width,
            num_layers,
            outputs: allocated_outputs,
            balancers: RawVec::with_capacity(num_layers * layer_width),
        };

        // For each layer in network, fill it with balancers. This method will allow easy
        // access to inputs in the traverse call.
        for index in (0..num_layers).rev() {
            let layer = network.layer_slice_mut(index);
            let mut unique_balancers: HashSet<*mut InternalBalancer<L>> = HashSet::new();

            for wire in wires.iter_mut() {
                if let Some((balancer, _)) = wire.pop() {
                    if !unique_balancers.contains(&balancer.as_ptr()) {
                        unique_balancers.insert(balancer.as_ptr());

                        layer[wire.value / 2] = balancer;
                    }
                }
            }
        }

        network
    }

    /// Returns the width of the network.
    ///
    /// This will always be a power of 2.
    pub fn width(&self) -> usize {
        self.width
    }

    fn num_layers(width: usize) -> usize {
        binomial_coefficient((log2_floor(width as u64) + 1) as u64, 2) as usize
    }

    /// Traverse the network and obtain a reference to an output element.
    pub fn traverse(&self) -> &L {
        let input_slot = hash_single(thread::current().id()) % (self.width as u64);

        let mut current: &Balancer<L> = unsafe { self.layer_slice(0)[input_slot as usize / 2].as_ref().next() };

        while let &Balancer::Internal(ref balancer) = current {
            current = unsafe { balancer.as_ref().next() };
        }

        assert!(current.is_leaf());
        unsafe { current.leaf_ref().as_ref() } 
    }

    fn layer_slice(&self, index: usize) -> &[NonNull<InternalBalancer<L>>] {
        let layer_width = self.width / 2;

        unsafe {
            let layer_head = self.balancers.ptr().add(layer_width * index);
            slice::from_raw_parts(layer_head, layer_width)
        }
    }

    fn layer_slice_mut(&mut self, index: usize) -> &mut [NonNull<InternalBalancer<L>>] {
        let layer_width = self.width / 2;

        unsafe {
            let layer_head = self.balancers.ptr().add(layer_width * index);
            slice::from_raw_parts_mut(layer_head, layer_width)
        }
    }
}

impl<L> Drop for BitonicNetwork<L> {
    fn drop(&mut self) {
        // Drop each internal balancer, leaving NonNull pointers to output
        // Then dealloc balancer memory
        let balancers_head = self.balancers.ptr();
        let balancer_layout = Layout::new::<InternalBalancer<L>>();
        for balancer_idx in 0..self.balancers.cap() {
            unsafe {
                balancers_head.add(balancer_idx).drop_in_place();
                Heap.dealloc(balancers_head.add(balancer_idx) as *mut u8, balancer_layout.clone());
            }
        }

        // For each output allocation, drop output and dealloc
        let output_layout = Layout::new::<L>();
        for output in self.outputs.iter_mut() {
            unsafe {
                output.as_ptr().drop_in_place();
                Heap.dealloc(output.as_ptr() as *mut u8, output_layout.clone());
            }
        }
    }
}

struct Wire<L> {
    balancer_history: Vec<(NonNull<InternalBalancer<L>>, bool)>,
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
        self.balancer_history.push((balancer, up));
    }

    fn pop(&mut self) -> Option<(NonNull<InternalBalancer<L>>, bool)> {
        self.balancer_history.pop()
    }
}

fn split_even_odd<L>(wires: Vec<Wire<L>>) -> (Vec<Wire<L>>, Vec<Wire<L>>) {
    let (even_wires, odd_wires): (Vec<(usize, Wire<L>)>, Vec<(usize, Wire<L>)>) = wires.into_iter().enumerate().partition(|&(idx, _)| idx % 2 == 0);

    let even = even_wires.into_iter().map(|(_, value)| value).collect::<Vec<_>>();

    let odd = odd_wires.into_iter().map(|(_, value)| value).collect::<Vec<_>>();

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
                let (mut last, up) = upper_wire.last();
                let mut temp = last.as_ptr().read();
                temp.outputs[up as usize] = Balancer::Internal(new_balancer_alloc);
                last.as_ptr().write(temp);
            }
        }

        upper_wire.add(new_balancer_alloc, true);
        lower_wire.add(new_balancer_alloc, false);

        wires.push(upper_wire);
        wires.push(lower_wire);
    }

    wires
}

fn construct_bitonic<L>(width: usize, wire_index: usize) -> Vec<Wire<L>> {
    if width == 1 {
        vec![Wire {
            balancer_history: Vec::new(),
            value: wire_index
        }]
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
