// this is a copy of the aesstream crate, with the only addition of impl EncryptWrite

use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};

use crate::EncryptWrite;
use rand::{rngs::OsRng, RngCore};
use rust_crypto::blockmodes::{CbcDecryptor, CbcEncryptor, DecPadding, EncPadding, PkcsPadding};
use rust_crypto::buffer::{BufferResult, ReadBuffer, RefReadBuffer, RefWriteBuffer, WriteBuffer};
use rust_crypto::symmetriccipher::{BlockDecryptor, BlockEncryptor, Decryptor, Encryptor};

const BUFFER_SIZE: usize = 8192;

pub struct AesWriter<E: BlockEncryptor, W: Write> {
    /// Writer to write encrypted data to
    writer: Option<W>,
    /// Encryptor to encrypt data with
    enc: CbcEncryptor<E, EncPadding<PkcsPadding>>,
    /// Indicates weather the encryptor has done its final operation (inserting padding)
    closed: bool,
}

impl<E: BlockEncryptor, W: Write> AesWriter<E, W> {
    pub fn new(mut writer: W, enc: E) -> Result<AesWriter<E, W>> {
        let mut iv = vec![0u8; enc.block_size()];
        OsRng.fill_bytes(&mut iv);
        writer.write_all(&iv)?;
        Ok(AesWriter {
            writer: Some(writer),
            enc: CbcEncryptor::new(enc, PkcsPadding, iv),
            closed: false,
        })
    }

    /// Encrypts passed buffer and writes all resulting encrypted blocks to the underlying writer
    ///
    /// # Parameters
    ///
    /// * **buf**: Plaintext to encrypt and write
    /// * **eof**: If the provided buf is the last one to come and therefore encryption should be
    ///     finished and padding added.
    fn encrypt_write(&mut self, buf: &[u8], eof: bool) -> Result<usize> {
        let mut read_buf = RefReadBuffer::new(buf);
        let mut out = [0u8; BUFFER_SIZE];
        let mut write_buf = RefWriteBuffer::new(&mut out);
        loop {
            let res = self
                .enc
                .encrypt(&mut read_buf, &mut write_buf, eof)
                .map_err(|e| Error::new(ErrorKind::Other, format!("encryption error: {:?}", e)))?;
            let mut enc = write_buf.take_read_buffer();
            let enc = enc.take_remaining();
            self.writer.as_mut().unwrap().write_all(enc)?;
            match res {
                BufferResult::BufferUnderflow => break,
                BufferResult::BufferOverflow if eof => panic!("read_buf underflow during encryption with eof"),
                BufferResult::BufferOverflow => {}
            }
        }
        // CbcEncryptor has its own internal buffer and always consumes all input
        assert_eq!(read_buf.remaining(), 0);
        Ok(buf.len())
    }
}

impl<E: BlockEncryptor, W: Write> Write for AesWriter<E, W> {
    /// Encrypts the passed buffer and writes the result to the underlying writer.
    ///
    /// Due to the blocksize of CBC not all data will be written instantaneously.
    /// For example if 17 bytes are passed, the first 16 will be encrypted as one block and written
    /// the underlying writer, but the last byte won't be encrypted and written yet.
    ///
    /// If [`flush`](#method.flush) has been called, this method will always return an error.
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if self.closed {
            return Err(Error::new(ErrorKind::Other, "AesWriter is closed"));
        }
        let written = self.encrypt_write(buf, false)?;
        Ok(written)
    }

    /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
    /// [Read more](https://doc.rust-lang.org/nightly/std/io/trait.Write.html#tymethod.flush)
    ///
    /// **Warning**: When this method is called, the encryption will finish and insert final padding.
    /// After calling `flush`, this writer cannot be written to anymore and will always return an
    /// error.
    fn flush(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        self.encrypt_write(&[], true)?;
        self.closed = true;
        self.writer.as_mut().unwrap().flush()
    }
}

impl<E: BlockEncryptor> EncryptWrite for AesWriter<E, Vec<u8>> {
    fn finalise(mut self) -> anyhow::Result<Vec<u8>> {
        self.flush()?;
        Ok(self.writer.take().unwrap())
    }
}

impl<E: BlockEncryptor, W: Write> Drop for AesWriter<E, W> {
    /// Drops this AesWriter trying to finish encryption and to write everything to the underlying writer.
    fn drop(&mut self) {
        if self.writer.is_some() {
            if !std::thread::panicking() {
                self.flush().unwrap();
            } else {
                let _ = self.flush();
            }
        }
    }
}

/// Wraps a [`Read`](https://doc.rust-lang.org/std/io/trait.Read.html) implementation with CBC
/// based on given [`BlockDecryptor`][bd]
///
/// [bd]: https://docs.rs/rust-crypto/0.2.36/crypto/symmetriccipher/trait.BlockDecryptor.html
///
pub struct AesReader<D: BlockDecryptor, R: Read> {
    /// Reader to read encrypted data from
    reader: R,
    /// Decryptor to decrypt data with
    dec: CbcDecryptor<D, DecPadding<PkcsPadding>>,
    /// Block size of BlockDecryptor, needed when seeking to correctly seek to the nearest block
    block_size: usize,
    /// Buffer used to store blob needed to find out if we reached eof
    buffer: Vec<u8>,
    /// Indicates wheather eof of the underlying buffer was reached
    eof: bool,
}

impl<D: BlockDecryptor, R: Read> AesReader<D, R> {
    /// Creates a new AesReader.
    ///
    /// Assumes that the first block of given reader is the IV.
    ///
    /// # Parameters
    ///
    /// * **reader**: Reader to read encrypted data from
    /// * **dec**: [`BlockDecryptor`][bd] to use for decyrption
    ///
    /// [bd]: https://docs.rs/rust-crypto/0.2.36/crypto/symmetriccipher/trait.BlockDecryptor.html
    pub fn new(mut reader: R, dec: D) -> Result<AesReader<D, R>> {
        let mut iv = vec![0u8; dec.block_size()];
        reader.read_exact(&mut iv)?;
        Ok(AesReader {
            reader,
            block_size: dec.block_size(),
            dec: CbcDecryptor::new(dec, PkcsPadding, iv),
            buffer: Vec::new(),
            eof: false,
        })
    }

    /// Reads at max BUFFER_SIZE bytes, handles potential eof and returns the buffer as Vec<u8>
    fn fill_buf(&mut self) -> Result<Vec<u8>> {
        let mut eof_buffer = vec![0u8; BUFFER_SIZE];
        let read = self.reader.read(&mut eof_buffer)?;
        self.eof = read == 0;
        eof_buffer.truncate(read);
        Ok(eof_buffer)
    }

    /// Reads and decrypts data from the underlying stream and writes it into the passed buffer.
    ///
    /// The CbcDecryptor has an internal output buffer, but not an input buffer.
    /// Therefore, we need to take care of letfover input.
    /// Additionally, we need to handle eof correctly, as CbcDecryptor needs to correctly interpret
    /// padding.
    /// Thus, we need to read 2 buffers. The first one is read as input for decryption and the second
    /// one to determine if eof is reached.
    /// The next time this function is called, the second buffer is passed as input into decryption
    /// and the first buffer is filled to find out if we reached eof.
    ///
    /// # Parameters
    ///
    /// * **buf**: Buffer to write decrypted data into.
    fn read_decrypt(&mut self, buf: &mut [u8]) -> Result<usize> {
        // if this is the first iteration, fill internal buffer
        if self.buffer.is_empty() && !self.eof {
            self.buffer = self.fill_buf()?;
        }

        let buf_len = buf.len();
        let mut write_buf = RefWriteBuffer::new(buf);
        let res;
        let remaining;
        {
            let mut read_buf = RefReadBuffer::new(&self.buffer);

            // test if CbcDecryptor still has enough decrypted data or we have enough buffered
            res = self
                .dec
                .decrypt(&mut read_buf, &mut write_buf, self.eof)
                .map_err(|e| Error::new(ErrorKind::Other, format!("decryption error: {:?}", e)))?;
            remaining = read_buf.remaining();
        }
        // keep remaining bytes
        let len = self.buffer.len();
        self.buffer.drain(..(len - remaining));
        // if we were able to decrypt, return early
        match res {
            BufferResult::BufferOverflow => return Ok(buf_len),
            BufferResult::BufferUnderflow if self.eof => return Ok(write_buf.position()),
            _ => {}
        }

        // else read new buffer
        let mut dec_len = 0;
        // We must return something, if we have something.
        // If the reader doesn't return enough so that we can decrypt a block, we need to continue
        // reading until we have enough data to return one decrypted block, or until we reach eof.
        // If we reach eof, we will be able to decrypt the final block because of padding.
        while dec_len == 0 && !self.eof {
            let eof_buffer = self.fill_buf()?;
            let remaining;
            {
                let mut read_buf = RefReadBuffer::new(&self.buffer);
                self.dec
                    .decrypt(&mut read_buf, &mut write_buf, self.eof)
                    .map_err(|e| Error::new(ErrorKind::Other, format!("decryption error: {:?}", e)))?;
                let mut dec = write_buf.take_read_buffer();
                let dec = dec.take_remaining();
                dec_len = dec.len();
                remaining = read_buf.remaining();
            }
            // keep remaining bytes
            let len = self.buffer.len();
            self.buffer.drain(..(len - remaining));
            // append newly read bytes
            self.buffer.extend(eof_buffer);
        }
        Ok(dec_len)
    }
}
impl<D: BlockDecryptor, R: Read + Seek> AesReader<D, R> {
    /// Seeks to *offset* from the start of the file
    fn seek_from_start(&mut self, offset: u64) -> Result<u64> {
        let block_num = offset / self.block_size as u64;
        let block_offset = offset % self.block_size as u64;
        // reset CbcDecryptor
        self.reader
            .seek(SeekFrom::Start((block_num - 1) * self.block_size as u64))?;
        let mut iv = vec![0u8; self.block_size];
        self.reader.read_exact(&mut iv)?;
        self.dec.reset(&iv);
        self.buffer = Vec::new();
        self.eof = false;
        let mut skip = vec![0u8; block_offset as usize];
        self.read_exact(&mut skip)?;
        // subtract IV
        Ok(offset - 16)
    }
}

impl<D: BlockDecryptor, R: Read> Read for AesReader<D, R> {
    /// Reads encrypted data from the underlying reader, decrypts it and writes the result into the
    /// passed buffer.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let read = self.read_decrypt(buf)?;
        Ok(read)
    }
}

impl<D: BlockDecryptor, R: Read + Seek> Seek for AesReader<D, R> {
    /// Seek to an offset, in bytes, in a stream.
    /// [Read more](https://doc.rust-lang.org/nightly/std/io/trait.Seek.html#tymethod.seek)
    ///
    /// When seeking, this reader takes care of reinitializing the CbcDecryptor with the correct IV.
    /// The passed position does *not* need to be aligned to the blocksize.
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        match pos {
            SeekFrom::Start(offset) => {
                // +16 because first block is the iv
                self.seek_from_start(offset + 16)
            }
            SeekFrom::End(_) | SeekFrom::Current(_) => {
                let pos = self.reader.seek(pos)?;
                self.seek_from_start(pos)
            }
        }
    }
}
