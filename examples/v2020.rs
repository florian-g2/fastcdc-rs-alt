//
// Copyright (c) 2023 Nathan Fiedler
//
use clap::{arg, command, value_parser, Arg};
use fastcdc_alt::*;
use memmap2::Mmap;
use std::fs::File;
use std::time::{Instant};

fn main() {
    let matches = command!("Example of using v2020 chunker.")
        .about("Finds the content-defined chunk boundaries of a file.")
        .arg(
            arg!(
                -s --size <SIZE> "The desired average size of the chunks."
            )
            .value_parser(value_parser!(u32)),
        )
        .arg(
            Arg::new("INPUT")
                .help("Sets the input file to use")
                .required(true)
                .index(1),
        )
        .get_matches();
    let size = matches.get_one::<u32>("size").unwrap_or(&131072);
    let avg_size = *size;
    let filename = matches.get_one::<String>("INPUT").unwrap();
    let file = File::open(filename).expect("cannot open file!");

    let start = Instant::now();
    let mmap = unsafe { Mmap::map(&file).expect("cannot create mmap?") };
    let min_size = avg_size / 4;
    let max_size = avg_size * 4;
    let mut chunker = FastCDC::new(min_size, avg_size, max_size).unwrap();
    chunker.set_content_length(mmap.len());

    for entry in chunker.as_iterator(&mmap) {
        println!(
            "hash={} offset={} size={}",
            entry.hash, entry.offset, entry.cutpoint
        );
    }

    println!("Finished in {}ms", start.elapsed().as_millis())
}
