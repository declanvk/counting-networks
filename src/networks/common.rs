use crate::util::{hash_single, slice_to_ptr_range};
use core::{
    any::type_name,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};
use std::thread;

#[cfg(all(test, loom))]
mod atomic {
    pub use loom::sync::atomic::{AtomicBool, Ordering};
}

#[cfg(not(all(test, loom)))]
mod atomic {
    pub use core::sync::atomic::{AtomicBool, Ordering};
}

use atomic::AtomicBool;

#[derive(Debug)]
pub enum WireSegment<L> {
    Balancer(Balancer<L>),
    End(*const L),
}

// Align struct to cache size (Intel)
// This prevents false sharing of the balancer between multiple cores.
#[repr(align(64))]
#[derive(Debug)]
pub struct Balancer<L> {
    pub value: AtomicBool,
    pub next_segments: [*const WireSegment<L>; 2],
}

impl<L> Balancer<L> {
    pub fn next_segment<'a>(&'a self) -> &'a WireSegment<L> {
        let next_index = self.toggle_up();
        // TODO: Write safety comment
        unsafe {
            self.next_segments
                .get_unchecked(next_index)
                .as_ref()
                .expect("pointer should never be null")
        }
    }

    // false -> 0, true -> 1
    pub fn toggle_up(&self) -> usize {
        self.value.fetch_xor(true, atomic::Ordering::Relaxed) as usize
    }
}

// A network is a configuration of balancers on a finite set of wires, so it can
// be described as a data structure with a predefined width and an iterator that
// yields the balancers in a special order.
//
// Given a width of 4 and the pairs `[(a,b), (c,d), (b,c), (a,d), (a,b), (c,d)]`
// the network below will be produced. The balancers are always given in a
// back-to-front, top-to-bottom order.
//
// Count depth from end of network to start of network. This will be used to
// label nodes uniquely. For example, the top wire in the network below will
// have 3 different counter values:
//
//
// ```text
//        3    2     1     0 - for the a & d wires
// a ─────╥────╥─────╥─── (o)
// b ─────╨────║──╥──╨─── (o)
// c ─────╥────║──╨──╥─── (o)
// d ─────╨────╨─────╨─── (o)
//        3       2  1     0 - for the b & c wires
// ```
//
// The following **nodes** will be produced from this network:
//
// ```text
// [(a1, b1), (c1, d1), (b2, c2), (a2, d2), (a3, b3), (c3, d3)]
// ```
//
// The higher wire is always ordered first in the tuple.
// The (labeled) edges in the (directed!) graph will continue to be the wires
// that connect them. For example, the edges for `(a2, b2)` (format is
// `(from_node, label, to_node)`: ```text
// [((a3, b3), a, (a2, d2)), ((a3, b3), b, (b2, c2))]
// ```

// Creates an `Iterator` that will yield balancers. Each balancer is a link
// between two wires, represented as two `usize` indexes.
pub trait NetworkConfiguration: IntoIterator<Item = (usize, usize)> {
    fn from_width(width: usize) -> Self;
}

pub struct Network<L, B> {
    // Marker for network Builder type
    _marker: PhantomData<B>,
    // Width of the network
    width: usize,
    // Outputs of the network
    outputs: Box<[L]>,
    // Pointers to segments' memory locations
    segments: Box<[WireSegment<L>]>,
    // Indices that point to the last segment for each wire, `len` should be equal to `width`.
    last_segments: Box<[usize]>,
}

impl<L, B: NetworkConfiguration> Network<L, B> {
    pub fn new(outputs: Vec<L>) -> Self {
        assert!(outputs.len() > 0);

        let outputs = outputs.into_boxed_slice();
        let width = outputs.len();
        let config = B::from_width(width);

        let mut next_segment_idx = width;
        let mut latest_segments: Vec<usize> = (0..width).collect();
        let mut balancers = Vec::new();

        // Populate a list of pairs of index pointers for the `next_segments` field of
        // `Balancers`
        for (top_wire, bottom_wire) in config {
            let balancer_ptrs = (latest_segments[top_wire], latest_segments[bottom_wire]);
            balancers.push(balancer_ptrs);

            latest_segments[top_wire] = next_segment_idx;
            latest_segments[bottom_wire] = next_segment_idx;
            next_segment_idx += 1;
        }

        // This `Vec` should never be resized.
        let mut segments = Vec::with_capacity(next_segment_idx);

        // Add the outputs to the segments
        segments.extend(
            outputs
                .iter()
                .map(|out_ref| WireSegment::End(out_ref as *const _))
                .rev(),
        );

        // Add the balancers to the segments
        for (top_segment_idx, bottom_balancer_idx) in balancers {
            let top_segment_ptr = &segments[top_segment_idx] as *const _;
            let bottom_segment_ptr = &segments[bottom_balancer_idx] as *const _;

            let new_balancer = Balancer {
                value: AtomicBool::new(true),
                next_segments: [top_segment_ptr, bottom_segment_ptr],
            };

            segments.push(WireSegment::Balancer(new_balancer));
        }

        // Check that all points in WireSegments (the `output` and `End` pointers) fall
        // within the bounds of either the `outputs` boxed slice or the `segments`
        // vector.
        debug_assert!(check_segment_ptrs_in_bounds(&segments, &outputs));
        debug_assert_eq!(segments.len(), next_segment_idx);

        Network {
            _marker: PhantomData,
            width,
            outputs,
            segments: segments.into_boxed_slice(),
            last_segments: latest_segments.into_boxed_slice(),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn traverse(&self) -> &L {
        let input_slot = (hash_single(thread::current().id()) as usize) % self.width;
        let start_segment_idx = self.last_segments[input_slot];
        let mut current_segment = &self.segments[start_segment_idx];

        while let WireSegment::Balancer(balancer) = current_segment {
            current_segment = balancer.next_segment();
        }

        match current_segment {
            WireSegment::End(output_ptr) => {
                // TODO: write unsafe explanation
                unsafe { output_ptr.as_ref().expect("pointer should never be null") }
            }
            WireSegment::Balancer(_) => unreachable!(
                "previous loop conditioned off of this variable not being a `Balancer`"
            ),
        }
    }

    pub fn outputs(&self) -> &[L] {
        &self.outputs
    }
}

impl<L: PartialEq, B> PartialEq for Network<L, B> {
    fn eq(&self, other: &Self) -> bool {
        self.outputs.eq(&other.outputs)
    }
}

impl<L: Eq, B> Eq for Network<L, B> {}

impl<L: Hash, B> Hash for Network<L, B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.width.hash(state);
        self.outputs.iter().for_each(|output| {
            output.hash(state);
        });
    }
}

impl<L: Clone, B: NetworkConfiguration> Clone for Network<L, B> {
    fn clone(&self) -> Self {
        Network::new(self.outputs.iter().cloned().collect())
    }
}

impl<L: fmt::Debug, B: fmt::Debug> fmt::Debug for Network<L, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Network")
            .field("width", &self.width)
            .field("outputs", &self.outputs)
            .field("balancer_config", &type_name::<B>())
            .finish()
    }
}

// TODO: Safety justification
unsafe impl<L: Send, B> Send for Network<L, B> {}
// TODO: Safety justification
unsafe impl<L: Sync, B> Sync for Network<L, B> {}

fn check_segment_ptrs_in_bounds<L>(segments: &[WireSegment<L>], outputs: &[L]) -> bool {
    let segments_range = slice_to_ptr_range(segments);
    let outputs_range = slice_to_ptr_range(outputs);

    segments.iter().all(|segment| match segment {
        WireSegment::Balancer(Balancer { next_segments, .. }) => {
            segments_range.contains(&next_segments[0]) && segments_range.contains(&next_segments[1])
        }
        WireSegment::End(output_ptr) => outputs_range.contains(output_ptr),
    })
}
