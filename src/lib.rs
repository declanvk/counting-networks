#![feature(allocator_api, pointer_methods)]
#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/log/0.1.1")]

//! A counting network is a type of concurrent data structure that gives non-blocking access to specific
//! operations, most commonly ``fetch-and-inc``.
//!
//! They were first introduced in a paper by James Aspnes, Maruice Herlihy, and Nir Shavit [\[1\]][original].
//!
//! # Construction
//!
//! Counting networks are from simple components called balancers. A balancer can be visualized as a
//! element that takes two input wires and produces two output wires. Threads moving through the
//! structure can be represented as tokens that pass along these wires. Balancers will alternate
//! sending tokens arriving on input wires to either the top output wire or the bottom output wire.
//! This ensures that the arriving tokens are distributed in a balanced way across the outputs.
//!
//! For example (where the number indicates the order that the tokens enter):
//! ```text
//! 7 6 4 2 1 ──╥── 1 3 5 7
//!       5 3 ──╨── 2 4 6
//! ```
//!
//! Counting networks are a subclass of a broader type of network called a balancer network that are
//! built using these elements. A balancer (and balancing networks in general), will ensure that the
//! difference in the number of tokens between each pair of wires is bounded. They are also known as
//! k-smoothing networks, where k is the bound [\[2\]][smoothing]. Counting networks are equivalent
//! to 1-smoothing networks, where the difference in outputs along each wire will always be 1. Another
//! property of counting networks is that they will always output tokens in increasing order modulo
//! the size of the network.
//!
//! A larger example illustrating this:
//! ```text
//!       ─────╥────╥─────╥─── 1 5
//! 4 3 1 ─────╨────║──╥──╨─── 2 6
//! 5     ─────╥────║──╨──╥─── 3 7
//! 7 6 2 ─────╨────╨─────╨─── 4
//! ```
//!
//! This can be applied to implement a shared counter, a data structure that provides ``fetch-and-inc``
//! operations, by attaching a local counter to each output wire so that every token leaving the network
//! will take the value of the counter and increment it by the width of the network. The i<sup>th</sup>
//! wire will have counter c<sub>*i*</sub> that initially contains value *i*. If *w* is the output
//! width of the network, the local counter will emit
//! the values *i*, *i* + *w*, *i* + 2*w*, *i* + 3*w*, ...
//!
//! # Papers
//!
//! Read the [paper by Aspnes et al.][original] as an introduction to the subject. There is also a
//! [portion of a textbook][textbook] that gives a lovely overview of concurrent data structures and
//! where counting networks fit in. Look for the section on Fetch-and-φ Structures. Another [paper][smoothing]
//! gives the larger context of balancing/smoothing networks. Lastly the
//! [Wikipedia page on sorting networks][wikipedia] is fairly intuitive, and you can see how they relate
//!  to the other types of networks.
//!
//! [original]: http://www.hpl.hp.com/techreports/Compaq-DEC/CRL-93-11.pdf
//! [textbook]: https://www.cs.tau.ac.il/~shanir/concurrent-data-structures.pdf
//! [smoothing]: http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.87.5843&rep=rep1&type=pdf
//! [wikipedia]: https://en.wikipedia.org/wiki/Sorting_network

pub mod networks;
pub mod counters;

mod util;