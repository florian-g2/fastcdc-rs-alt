//
// Copyright (c) 2023 Nathan Fiedler
// Copyright (c) 2023 Florian Gäbler
//

//! # FastCDC-Alt
//!
//! This crate is a fork of the original Rust implementation by Nathan Fiedler. ([nlfiedler/fastcdc-rs](https://github.com/nlfiedler/fastcdc-rs))\
//! Included here is the enhanced 2020 "FastCDC" content defined chunking algorithm described by Wen Xia, et al.\
//! \
//! This fork introduces an adjusted and a bit more complicated alternative API to increase the flexibility and reduce the memory overhead.\
//! The cut points produced by this fork are identical to the ones produced by the original crate.\
//! \
//! The README and all docs are adapted to the adjusted API.\
//! \
//! To learn more about content defined chunking and its applications, see
//! [FastCDC: a Fast and Efficient Content-Defined Chunking Approach for Data
//! Deduplication](https://www.usenix.org/system/files/conference/atc16/atc16-paper-xia.pdf)
//! from 2016, as well as the subsequent improvements described in the
//! [paper](https://ieeexplore.ieee.org/document/9055082) from 2020\
//!
//! ## Examples
//!
//! A short example of using the fast chunker is shown below:
//!
//! ```no_run
//! use std::fs;
//! use fastcdc_alt::FastCDC;
//! let contents = fs::read("test/fixtures/SekienAkashita.jpg").unwrap();
//! let mut chunker = FastCDC::new(4096, 16384, 65535).unwrap();
//!
//! for chunk in chunker.as_iterator(&contents) {
//!     println!("offset={} size={}", chunk.offset, chunk.get_length());
//! }
//! ```
//!
//! The example above is using normalized chunking level 1 as described in
//! section 3.5 of the 2020 paper. To use a different level of chunking
//! normalization, replace `new` with `new_advanced` as shown below:
//!
//! ```no_run
//! use std::fs;
//! use fastcdc_alt::{FastCDC, Normalization};
//! let contents = fs::read("test/fixtures/SekienAkashita.jpg").unwrap();
//! let mut chunker = FastCDC::new_advanced(8192, 16384, 32768, Normalization::Level3, None).unwrap();
//! for chunk in chunker.as_iterator(&contents) {
//!     println!("offset={} size={}", chunk.offset, chunk.get_length());
//! }
//! ```
//!
//! Notice that the minimum and maximum chunk sizes were changed in the example
//! using the maximum normalized chunking level. This is due to the behavior of
//! normalized chunking in which the generated chunks tend to be closer to the
//! expected chunk size. It is not necessary to change the min/max values, just
//! something of which to be aware. With lower levels of normalized chunking,
//! the size of the generated chunks will vary more. See the documentation of
//! the `Normalization` enum for more detail as well as the FastCDC paper.
//!
//! ## Minimum and Maximum
//!
//! The values you choose for the minimum and maximum chunk sizes will depend on
//! the input data to some extent, as well as the normalization level described
//! above. Depending on your application, you may want to have a wide range of
//! chunk sizes in order to improve the overall deduplication ratio.
//!
//! Note that changing the minimum chunk size will almost certainly result in
//! different cut points. It is best to pick a minimum chunk size for your
//! application that can remain relevant indefinitely, lest you produce
//! different sets of chunks for the same data.
//!
//! Similarly, setting the maximum chunk size to be too small may result in cut
//! points that were determined by the maximum size rather than the data itself.
//! Ideally you want cut points that are determined by the input data. However,
//! this is application dependent and your situation may be different.
//!
//! ## Large Data
//!
//! If processing very large files, the streaming version of the chunker may be
//! a suitable approach. It allocate a byte vector equal to the maximum
//! chunk size, draining and resizing the vector as chunks are found. However,
//! using a crate such as `memmap2` can be significantly faster than the streaming
//! chunker. See the examples in the `examples` directory for how to use the
//! streaming versions as-is, versus the non-streaming chunkers which read from a
//! memory-mapped file.
//! Also consider directly leveraging the `cut()` method of the FastCDC struct to
//! manually implement a streaming functionality.

pub mod v2020;

pub use v2020::*;