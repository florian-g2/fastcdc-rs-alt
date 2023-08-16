//
// Copyright (c) 2023 Nathan Fiedler
// Copyright (c) 2023 Florian Gäbler
//

use super::*;

#[cfg(all(feature = "futures", not(feature = "tokio")))]
use futures::{
    io::{AsyncRead, AsyncReadExt},
    stream::Stream,
};

#[cfg(all(feature = "tokio", not(feature = "futures")))]
use tokio_stream::Stream;

#[cfg(all(feature = "tokio", not(feature = "futures")))]
use tokio::io::{AsyncRead, AsyncReadExt};

#[cfg(all(feature = "tokio", not(feature = "futures")))]
use async_stream::try_stream;

///
/// An async-streamable version of the FastCDC chunker implementation from 2020
/// with streaming support.
///
/// Use `new` to construct an instance, and then `as_stream` to produce an async
/// [Stream] of the chunks.
///
/// Both `futures` and `tokio`-based [AsyncRead] inputs are supported via
/// feature flags. But, if necessary you can also use the
/// [`async_compat`](https://docs.rs/async-compat/latest/async_compat/) crate to
/// adapt your inputs as circumstances may require.
///
/// Note that this struct allocates a `Vec<u8>` of `max_size` bytes to act as a
/// buffer when reading from the source and finding chunk boundaries.
///
/// ```no_run
/// # use std::fs::File;
/// # use fastcdc_alt::v2020::AsyncStreamCDC;
/// # #[cfg(all(feature = "futures", not(feature = "tokio")))]
/// # use futures::stream::StreamExt;
/// # #[cfg(all(feature = "tokio", not(feature = "futures")))]
/// # use tokio_stream::StreamExt;
///
/// async fn run() {
///     let source = std::fs::read("test/fixtures/SekienAkashita.jpg").unwrap();
///     let mut chunker = AsyncStreamCDC::new(source.as_ref(), 4096, 16384, 65535).unwrap();
///     let stream = chunker.as_stream();
///
///     let chunks = stream.collect::<Vec<_>>().await;
///
///     for result in chunks {
///         let (_data, chunk) = result.unwrap();
///         println!("offset={} length={}", chunk.offset, chunk.cutpoint);
///     }
/// }
/// ```
///
pub struct AsyncStreamCDC<R> {
    inner: FastCDC,
    /// Buffer of data from source for finding cut points.
    buffer: Vec<u8>,
    /// Maximum capacity of the buffer (always `max_size`).
    capacity: usize,
    /// Number of relevant bytes in the `buffer`.
    length: usize,
    /// Source from which data is read into `buffer`.
    source: R,
    /// Number of bytes read from the source so far.
    processed: usize,
    /// True when the source produces no more data.
    eof: bool,
}

impl<R: AsyncRead + Unpin> AsyncStreamCDC<R> {
    ///
    /// Construct a `StreamCDC` that will process bytes from the given source.
    ///
    /// Uses chunk size normalization level 1 by default.
    ///
    pub fn new(source: R, min_size: u32, avg_size: u32, max_size: u32) -> Result<Self, Error> {
        Self::new_advanced(source, min_size, avg_size, max_size, Normalization::Level1)
    }

    ///
    /// Create a new `StreamCDC` with the given normalization level.
    ///
    pub fn new_advanced(
        source: R,
        min_size: u32,
        avg_size: u32,
        max_size: u32,
        level: Normalization,
    ) -> Result<Self, Error> {
        Ok(Self {
            inner: FastCDC::new_advanced(min_size, avg_size, max_size, level, None)?,
            buffer: vec![0; max_size as usize],
            capacity: max_size as usize,
            length: 0,
            source,
            processed: 0,
            eof: false,
        })
    }

    /// Fill the buffer with data from the source, returning the number of bytes
    /// read (zero if end of source has been reached).
    async fn fill_buffer(&mut self) -> Result<usize, Error> {
        // this code originally copied from asuran crate
        if self.eof {
            Ok(0)
        } else {
            let mut all_bytes_read = 0;
            while !self.eof && self.length < self.capacity {
                let bytes_read = self.source.read(&mut self.buffer[self.length..]).await?;
                if bytes_read == 0 {
                    self.eof = true;
                } else {
                    self.length += bytes_read;
                    all_bytes_read += bytes_read;
                }
            }
            Ok(all_bytes_read)
        }
    }

    /// Drains a specified number of bytes from the buffer, then resizes the
    /// buffer back to `capacity` size in preparation for further reads.
    fn drain_bytes(&mut self, count: usize) -> Result<Vec<u8>, Error> {
        // this code originally copied from asuran crate
        if count > self.length {
            Err(Error::Other(format!(
                "drain_bytes() called with count larger than length: {} > {}",
                count, self.length
            )))
        } else {
            let data = self.buffer.drain(..count).collect::<Vec<u8>>();
            self.length -= count;
            self.buffer.resize(self.capacity, 0_u8);
            Ok(data)
        }
    }

    /// Find the next chunk in the source. If the end of the source has been
    /// reached, returns `Error::Empty` as the error.
    async fn read_chunk(&mut self) -> Result<(Vec<u8>, Chunk), Error> {
        self.fill_buffer().await?;
        if self.length == 0 {
            Err(Error::Empty)
        } else {
            self.inner.set_content_length(self.length);

            let chunk = self.inner.cut(&self.buffer[..self.length]).ok_or(Error::Empty)?;
            let data = self.drain_bytes(chunk.cutpoint)?;

            let cutpoint = self.processed + chunk.cutpoint;
            let chunk = Chunk {
                hash: chunk.hash,
                offset: self.processed as isize,
                cutpoint
            };

            self.processed = cutpoint;

            Ok((data, chunk))
        }
    }

    #[cfg(all(feature = "tokio", not(feature = "futures")))]
    pub fn as_stream(&mut self) -> impl Stream<Item = Result<(Vec<u8>, Chunk), Error>> + '_ {
        try_stream! {
            loop {
                match self.read_chunk().await {
                    Ok(tuple) => yield tuple,
                    Err(Error::Empty) => {
                        break;
                    }
                    error @ Err(_) => {
                        error?;
                    }
                }
            }
        }
    }

    #[cfg(all(feature = "futures", not(feature = "tokio")))]
    pub fn as_stream(&mut self) -> impl Stream<Item = Result<(Vec<u8>, Chunk), Error>> + '_ {
        futures::stream::unfold(self, |this| async {
            let tuple = this.read_chunk().await;
            if let Err(Error::Empty) = tuple {
                None
            } else {
                Some((tuple, this))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::AsyncStreamCDC;

    #[test]
    fn test_minimum_too_low() {
        let array = [0u8; 1024];
        assert!(AsyncStreamCDC::new(array.as_slice(), 63, 256, 1024).is_err());
    }

    #[test]
    fn test_minimum_too_high() {
        let array = [0u8; 1024];
        assert!(AsyncStreamCDC::new(array.as_slice(), 67_108_867, 256, 1024).is_err());
    }

    #[test]
    fn test_average_too_low() {
        let array = [0u8; 1024];
        assert!(AsyncStreamCDC::new(array.as_slice(), 64, 255, 1024).is_err());
    }

    #[test]
    fn test_average_too_high() {
        let array = [0u8; 1024];
        assert!(AsyncStreamCDC::new(array.as_slice(), 64, 268_435_457, 1024).is_err());
    }

    #[test]
    fn test_maximum_too_low() {
        let array = [0u8; 1024];
        assert!(AsyncStreamCDC::new(array.as_slice(), 64, 256, 1023).is_err());
    }

    #[test]
    fn test_maximum_too_high() {
        let array = [0u8; 1024];
        assert!(AsyncStreamCDC::new(array.as_slice(), 64, 256, 1_073_741_825).is_err());
    }

    struct ExpectedChunk {
        hash: u64,
        offset: u64,
        length: usize,
        digest: String,
    }

    use md5::{Digest, Md5};

    #[cfg(all(feature = "futures", not(feature = "tokio")))]
    use futures::stream::StreamExt;
    #[cfg(all(feature = "tokio", not(feature = "futures")))]
    use tokio_stream::StreamExt;

    #[cfg_attr(all(feature = "tokio", not(feature = "futures")), tokio::test)]
    #[cfg_attr(all(feature = "futures", not(feature = "tokio")), futures_test::test)]
    async fn test_iter_sekien_16k_chunks() {
        let read_result = std::fs::read("test/fixtures/SekienAkashita.jpg");
        assert!(read_result.is_ok());
        let contents = read_result.unwrap();
        // The digest values are not needed here, but they serve to validate
        // that the streaming version tested below is returning the correct
        // chunk data on each iteration.
        let expected_chunks = vec![
            ExpectedChunk {
                hash: 17968276318003433923,
                offset: 0,
                length: 21325,
                digest: "2bb52734718194617c957f5e07ee6054".into(),
            },
            ExpectedChunk {
                hash: 8197189939299398838,
                offset: 21325,
                length: 17140,
                digest: "badfb0757fe081c20336902e7131f768".into(),
            },
            ExpectedChunk {
                hash: 13019990849178155730,
                offset: 38465,
                length: 28084,
                digest: "18412d7414de6eb42f638351711f729d".into(),
            },
            ExpectedChunk {
                hash: 4509236223063678303,
                offset: 66549,
                length: 18217,
                digest: "04fe1405fc5f960363bfcd834c056407".into(),
            },
            ExpectedChunk {
                hash: 2504464741100432583,
                offset: 84766,
                length: 24700,
                digest: "1aa7ad95f274d6ba34a983946ebc5af3".into(),
            },
        ];
        let mut chunker = AsyncStreamCDC::new(contents.as_ref(), 4096, 16384, 65535).unwrap();
        let stream = chunker.as_stream();

        let chunks = stream.collect::<Vec<_>>().await;

        let mut index = 0;

        for chunk in chunks {
            let (_data, chunk) = chunk.unwrap();
            assert_eq!(chunk.hash, expected_chunks[index].hash);
            assert_eq!(chunk.offset, expected_chunks[index].offset as isize);
            assert_eq!(chunk.cutpoint, expected_chunks[index].offset as usize + expected_chunks[index].length);
            let mut hasher = Md5::new();
            hasher
                .update(&contents[chunk.offset as usize..chunk.cutpoint]);
            let table = hasher.finalize();
            let digest = format!("{:x}", table);
            assert_eq!(digest, expected_chunks[index].digest);
            index += 1;
        }
        assert_eq!(index, 5);
    }
}
