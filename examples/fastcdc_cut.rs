//
// Copyright (c) 2023 Florian Gäbler
//

use fastcdc_alt::{FastCDC};

fn main() {
    let buffers = vec![
        vec![0; 1024],
        vec![0; 1024],
        vec![0; 1024],
        vec![0; 1024]
    ];

    let mut chunker = FastCDC::new(64, 1024, 2560).unwrap();
    chunker.set_content_length(4096);

    for (i, buffer) in buffers.iter().enumerate() {
        let mut cursor = 0;
        loop {
            // Buffer 1 with cursor at 0 returned None.
            // Buffer 2 with cursor at 0 returned None.
            // Buffer 3 with cursor at 0 returned cut point 2560.
            // Buffer 3 with cursor at 512 returned None.
            // Buffer 4 with cursor at 0 returned cut point 1536.
            if let Some(chunk) = chunker.cut(&buffer[cursor..]) {
                println!("Buffer {} with cursor at {} returned cut point {}.", i + 1, cursor, chunk.cutpoint);

                cursor += chunk.cutpoint_in_buffer;
                if cursor == buffer.len() { break }
            } else {
                println!("Buffer {} with cursor at {} returned None.", i + 1, cursor);
                break;
            }
        }
    }
}