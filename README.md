# FastCDC-Alt [![docs.rs](https://docs.rs/fastcdc-alt/badge.svg)](https://docs.rs/fastcdc-alt) [![Crates.io](https://img.shields.io/crates/v/fastcdc-alt.svg)](https://crates.io/crates/fastcdc-alt) ![Test](https://github.com/florian-g2/fastcdc-rs-alt/actions/workflows/test.yml/badge.svg)

This crate is a fork of the original Rust implementation by Nathan Fiedler. ([nlfiedler/fastcdc-rs](https://github.com/nlfiedler/fastcdc-rs))\
Included here is the enhanced 2020 "FastCDC" content defined chunking algorithm described by Wen Xia, et al.\
\
This fork introduces an adjusted and a bit more complicated alternative API to increase the flexibility and reduce the memory overhead.\
The cut points produced by this fork are identical to the ones produced by the original crate.\
\
This README and all docs are adapted to the adjusted API.

## Whats different?
The adjusted `FastCDC` structure now allows you to provide the data piece by piece required to find the next cut point.\
The advantages of this approach are that it is no more required to keep the entire data to cut in one contiguous memory block and therefore also save on some memory copies.\
This is most useful for e.g. advanced streaming logics.

Example usage with the original crate:
```rust
fn main(consumer: Receiver<Box<[u8]>>) {

  let cursor = 0;
  let mut intermediate_buffer = vec![1024 * 1024 * 16]; // 16 MiB
  for buffer in consumer.iter() {
    &intermediate_buffer[cursor..cursor + 4096].copy_from_slice(&buffer);
    
    cursor += 4096;
    if cursor == intermediate_buffer.len() {
      
      let fastcdc = FastCDC::new(&intermediate_buffer, 65535, 1048576, 16777216);
      for chunk in fastcdc {
        // .. process chunk
      }

      cursor = 0;
    }
  }
}
```

Very basic example usage with this fork.\
You can see a full example in the [fastcdc_cut](/examples/fastcdc_cut.rs) file.

```rust
use std::collections::VecDeque;

fn main(consumer: Receiver<Box<[u8]>>) {
  let mut fastcdc = FastCDC::new(65535, 1048576, 16777216).unwrap();

  // Inform the FastCDC struct how much data we are expecting.
  fastcdc.set_content_length(134217728); // 128 MiB
  

  for buffer in consumer.iter() {
    let mut cursor = 0;
    
    loop {      
      if let Some(chunk) = fastcdc.cut(&buffer[cursor..]) {
        // .. process chunk hash

        cursor += chunk.cutpoint;
        if cursor == buffer.len() {
          break;
        }
      }
        break;
      }
    }
  }
}
```

---

*What else?*
* The `FastCDC` iterator is now accessible using the `FastCDC::as_iterator(&self, buffer: &[u8])` method.
* The `AsyncStreamCDC` and `StreamCDC` implementations have been adapted, their APIs changed just a little bit.
* To focus solely on the 2020 version in this fork, the *ronomon* and *v2016* implementations and examples have been removed.

## Requirements

* [Rust](https://www.rust-lang.org) stable (2018 edition)

## Building and Testing

```shell
$ cargo clean
$ cargo build
$ cargo test
```

## Example Usage

Examples can be found in the `examples` directory of the source repository, which demonstrate finding chunk boundaries in a given file. There are both streaming and non-streaming examples, where the non-streaming examples use the `memmap2` crate to read large files efficiently.

```shell
$ cargo run --example v2020 -- --size 16384 test/fixtures/SekienAkashita.jpg
    Finished dev [unoptimized + debuginfo] target(s) in 0.03s
     Running `target/debug/examples/v2020 --size 16384 test/fixtures/SekienAkashita.jpg`
hash=17968276318003433923 offset=0 size=21325
hash=4098594969649699419 offset=21325 size=17140
hash=15733367461443853673 offset=38465 size=28084
hash=4509236223063678303 offset=66549 size=18217
hash=2504464741100432583 offset=84766 size=24700
```

### Non-streaming

An example using `FastCDC` to find chunk boundaries in data loaded into memory:

```rust
let contents = std::fs::read("test/fixtures/SekienAkashita.jpg").unwrap();
let mut chunker = fastcdc_alt::FastCDC::new(16384, 32768, 65536);
for chunk in chunker.as_iterator(&contents) {
    println!("offset={} length={}", chunk.offset, chunk.length);
}
```

### Streaming

The `StreamCDC` version takes a `Read` source
and uses a byte vector with capacity equal to the specified maximum chunk size.

```rust
let source = std::fs::File::open("test/fixtures/SekienAkashita.jpg").unwrap();
let chunker = fastcdc_alt::StreamCDC::new(source, 4096, 16384, 65535).unwrap();
for result in chunker {
  let (_data, chunk) = result.unwrap();
  println!("offset={} length={}", chunk.offset, chunk.cutpoint);
}
```

### Async Streaming
There is also an async streaming version of FastCDC named `AsyncStreamCDC`,
which takes an `AsyncRead` (both `tokio` and `futures` are supported via feature flags)
and uses a byte vector with capacity equal to the specified maximum chunk size.

```rust
let source = std::fs::File::open("test/fixtures/SekienAkashita.jpg").unwrap();
let chunker = fastcdc_alt::AsyncStreamCDC::new(&source, 4096, 16384, 65535);
let stream = chunker.as_stream();
let chunks = stream.collect::<Vec<_>>().await;

for result in chunks {
  let chunk = result.unwrap();
  println!("offset={} length={}", chunk.offset, chunk.length);
}
```
## Reference Material

The original algorithm from 2016 is described in [FastCDC: a Fast and Efficient Content-Defined Chunking Approach for Data Deduplication](https://www.usenix.org/system/files/conference/atc16/atc16-paper-xia.pdf).\
The improved "rolling two bytes each time" version from 2020 is detailed in [The Design of Fast Content-Defined Chunking for Data Deduplication Based Storage Systems](https://ieeexplore.ieee.org/document/9055082).

## Other Implementations
* [nlfiedler/fastcdc-rs](https://github.com/nlfiedler/fastcdc-rs)
  + The base of this fork.
* [jrobhoward/quickcdc](https://github.com/jrobhoward/quickcdc)
  + Similar but slightly earlier algorithm by some of the same authors?
* [rdedup_cdc at docs.rs](https://docs.rs/crate/rdedup-cdc/0.1.0/source/src/fastcdc.rs)
  + Alternative implementation in Rust.
* [ronomon/deduplication](https://github.com/ronomon/deduplication)
  + C++ and JavaScript implementation of a variation of FastCDC.
* [titusz/fastcdc-py](https://github.com/titusz/fastcdc-py)
  + Pure Python port of FastCDC. Compatible with this implementation.
* [wxiacode/FastCDC-c](https://github.com/wxiacode/FastCDC-c)
  + Canonical algorithm in C with gear table generation and mask values.
* [wxiacode/restic-FastCDC](https://github.com/wxiacode/restic-FastCDC)
  + Alternative implementation in Go with additional mask values.
