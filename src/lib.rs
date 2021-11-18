use rand::rngs::ThreadRng;
use rand::Rng;
use std::fs::File;
use std::fs::Metadata;
use std::io::{Read, Result, Seek, SeekFrom, Write};
use std::path::Path;

//! Rust's std::io::Read and std::io::Write traits both document that the read
//! and write functions can incompletely fill the buffer, but this case is rare.
//! Code must be written to handle this case and this can go untested.
//!
//! This crate provides "IncompleteFile" that truncates the read and write size
//! and allows testing of those code paths.

pub struct IncompleteFile {
    file: File,
    rng: ThreadRng,
}

impl IncompleteFile {
    #[allow(dead_code)]
    pub fn create(path: &dyn AsRef<Path>) -> Result<Self> {
        let rng = rand::thread_rng();
        Ok(Self {
            file: File::create(&path)?,
            rng,
        })
    }

    #[allow(dead_code)]
    pub fn open(path: &dyn AsRef<Path>) -> Result<Self> {
        let rng = rand::thread_rng();
        Ok(Self {
            file: File::open(&path)?,
            rng,
        })
    }

    #[allow(dead_code)]
    pub fn metadata(&self) -> Result<Metadata> {
        self.file.metadata()
    }
}

impl Read for IncompleteFile {
    /**
     * Rust's std::io::Read trait documentation says:
     *
     *  If the return value of this method is Ok(n), then implementations must
     *  guarantee that 0 <= n <= buf.len().
     *
     * Exercise that code path by truncating the read size here from 1 ->
     * buf.len() (because 0 would be EOF).
     */
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() == 1 {
            return self.file.read(buf);
        }

        let truncated_size = self.rng.gen_range(1..buf.len());
        self.file.read(&mut buf[0..truncated_size])
    }
}

impl Write for IncompleteFile {
    /**
     * Rust's std::io::Write trait documentation says:
     *
     *  If the return value is Ok(n) then it must be guaranteed that n <=
     *  buf.len().
     *
     * Exercise that code path by truncating the write size here from 1 ->
     * buf.len().
     */
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if buf.len() == 1 {
            return self.file.write(buf);
        }

        let truncated_size = self.rng.gen_range(1..buf.len());
        self.file.write(&buf[0..truncated_size])
    }

    // Pass-through flush
    fn flush(&mut self) -> Result<()> {
        self.file.flush()
    }
}

impl Seek for IncompleteFile {
    // Pass-through seek
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.file.seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    fn read_test_i(sz: usize) -> Result<()> {
        // write out random data
        let mut random_data = vec![0; sz];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut random_data);

        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(&random_data)?;

        let mut reader = IncompleteFile::open(&file.path())?;

        // read it all in in in chunks
        let mut offset = 0;

        loop {
            let mut buffer = vec![0; 320];
            let mut total_read = 0;

            loop {
                assert!(total_read <= 320);
                if total_read == 320 {
                    break;
                }

                let n = reader.read(&mut buffer[total_read..])?;
                if n == 0 {
                    // EOF
                    break;
                }

                total_read += n;
            }

            assert_eq!(total_read, 320);

            assert_eq!(random_data[offset..(offset + 320)], buffer,);

            offset += 320;

            if offset == sz {
                break;
            }
        }

        Ok(())
    }

    #[test]
    fn read_test() -> Result<()> {
        for sz in 1..64 {
            read_test_i(320 * sz)?;
        }

        Ok(())
    }

    fn write_test_i(sz: usize) -> Result<()> {
        // write out random data
        let mut random_data = vec![0; sz];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut random_data);

        let dir = tempfile::tempdir()?;
        let path = dir.path().join("random");

        let mut writer = IncompleteFile::create(&path)?;
        writer.write_all(&random_data)?;

        // read back and compare

        let actual_data = std::fs::read(&path)?;

        assert_eq!(random_data, actual_data);

        Ok(())
    }

    #[test]
    fn write_test() -> Result<()> {
        for sz in 1..64 {
            write_test_i(320 * sz)?;
        }

        Ok(())
    }
}
