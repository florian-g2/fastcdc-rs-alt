//
// Copyright (c) 2023 Florian Gäbler
//

use std::fs;
use md5::{Digest, Md5};
use md5::digest::{FixedOutputReset};
use fastcdc_alt::FastCDC;

fn main() {
    let mut hasher = Md5::new();
    let mut fastcdc = FastCDC::new(4096, 16384, 65535).unwrap();

    let file_content = fs::read("test/fixtures/SekienAkashita.jpg").unwrap();
    let file_size = file_content.len();

    // Inform the FastCDC struct how much data we are expecting.
    fastcdc.set_content_length(file_size); // 128 MiB

    let buffers = file_content.chunks(4096).map(|slice| Vec::from(slice)).collect::<Vec<_>>();

    // Hold buffers here as long they are not completely included in chunks
    let mut uncompleted_buffers = Vec::<Vec<u8>>::new();
    for buffer in buffers {
        let mut cursor = 0;

        loop {
            if let Some(chunk) = fastcdc.cut(&buffer[cursor..]) {
                // if chunk starts at a previous buffer
                if chunk.offset < 0 {
                    // e.g. -212 means that the last 212 bytes in the previous buffer are part of this chunk.
                    let bytes_in_previous = (chunk.offset * -1) as usize;

                    for (i, buffer) in uncompleted_buffers.drain(..).enumerate() {
                        // if this is the first buffer, get the chunk start using below calculation
                        let start = if i == 0 { buffer.len() - bytes_in_previous % buffer.len() } else { 0 };

                        // process data of previous buffer
                        let data = &buffer[start..];
                        hasher.update(data);
                    }
                }

                // process data of current buffer
                let data = &buffer[cursor..cursor + chunk.cutpoint];
                hasher.update(data);

                let table = hasher.finalize_fixed_reset();
                let digest = format!("{:x}", table);

                println!(
                    "hash={} offset={} size={} digest={}",
                    chunk.hash, chunk.offset, chunk.get_length(), digest
                );

                // advance the cursor
                cursor += chunk.cutpoint;

                // break if the buffer is completely chunked
                if cursor == buffer.len() {
                    // .. the buffer is dropped here
                    // .. this would be the place where you could send the buffer to a next consumer

                    break;
                }
            } else {
                // buffer has not been completely chunked, so add it to the array.
                // we don't need to keep track of the cursor here.
                // if a later cut() finds a chunk, the offset is set to a negative value indicating where the chunk starts in this previous buffer.
                uncompleted_buffers.push(buffer);

                break;
            }
        }
    }
}