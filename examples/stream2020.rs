//
// Copyright (c) 2023 Nathan Fiedler
//
use clap::{arg, command, value_parser, Arg};
use fastcdc_alt::*;
use std::fs::File;

fn main() {
    let matches = command!("Example of using v2020 streaming chunker.")
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
    let min_size = avg_size / 4;
    let max_size = avg_size * 4;
    let chunker = StreamCDC::new(file, min_size, avg_size, max_size).unwrap();
    for result in chunker {
        let (_data, chunk) = result.expect("failed to read chunk");
        println!(
            "hash={} offset={} size={}",
            chunk.hash, chunk.offset, chunk.get_length()
        );
    }
}
