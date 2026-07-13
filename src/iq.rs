use crate::complex::Complex32;
use crate::{Error, Result};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IqFormat {
    F32Le,
    S16Le,
}

impl IqFormat {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "f32le" | "cf32" => Ok(Self::F32Le),
            "s16le" | "cs16" => Ok(Self::S16Le),
            _ => Err(Error::InvalidConfiguration(format!(
                "unsupported I/Q format {value:?}; expected f32le or s16le"
            ))),
        }
    }

    const fn bytes_per_sample(self) -> usize {
        match self {
            Self::F32Le => 8,
            Self::S16Le => 4,
        }
    }
}

pub struct IqReader<R: Read> {
    reader: R,
    format: IqFormat,
    next_sample_index: u64,
    finished: bool,
}

impl<R: Read> IqReader<R> {
    pub fn new(reader: R, format: IqFormat) -> Self {
        Self {
            reader,
            format,
            next_sample_index: 0,
            finished: false,
        }
    }

    pub const fn next_sample_index(&self) -> u64 {
        self.next_sample_index
    }

    pub fn read_block(&mut self, maximum_samples: usize) -> Result<Vec<Complex32>> {
        if maximum_samples == 0 {
            return Err(Error::InvalidConfiguration(
                "maximum_samples must be greater than zero".to_owned(),
            ));
        }
        if self.finished {
            return Ok(Vec::new());
        }

        let bytes_per_sample = self.format.bytes_per_sample();
        let maximum_bytes = maximum_samples
            .checked_mul(bytes_per_sample)
            .ok_or_else(|| {
                Error::InvalidConfiguration("I/Q block allocation size overflow".to_owned())
            })?;
        let mut bytes = vec![0u8; maximum_bytes];
        let mut length = 0usize;
        while length < maximum_bytes {
            match self.reader.read(&mut bytes[length..]) {
                Ok(0) => {
                    self.finished = true;
                    break;
                }
                Ok(count) => length += count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }
        bytes.truncate(length);
        if !length.is_multiple_of(bytes_per_sample) {
            return Err(Error::InvalidInput(format!(
                "I/Q stream ended with {length} bytes in the final block, which is not divisible by the {bytes_per_sample}-byte sample size"
            )));
        }

        let samples = parse_iq_bytes(&bytes, self.format)?;
        self.next_sample_index = self
            .next_sample_index
            .checked_add(samples.len() as u64)
            .ok_or_else(|| Error::InvalidInput("I/Q sample index overflow".to_owned()))?;
        Ok(samples)
    }
}

pub fn open_iq_file(
    path: impl AsRef<Path>,
    format: IqFormat,
) -> Result<(IqReader<BufReader<File>>, usize)> {
    let file = File::open(path.as_ref())?;
    let length_u64 = file.metadata()?.len();
    let length = usize::try_from(length_u64).map_err(|_| {
        Error::InvalidInput(format!(
            "I/Q file length {length_u64} cannot be represented on this platform"
        ))
    })?;
    let bytes_per_sample = format.bytes_per_sample();
    if !length.is_multiple_of(bytes_per_sample) {
        return Err(Error::InvalidInput(format!(
            "I/Q file length {length} is not divisible by the {bytes_per_sample}-byte sample size"
        )));
    }
    Ok((
        IqReader::new(BufReader::new(file), format),
        length / bytes_per_sample,
    ))
}

pub fn read_iq_file(
    path: impl AsRef<Path>,
    format: IqFormat,
    max_samples: usize,
) -> Result<Vec<Complex32>> {
    if max_samples == 0 {
        return Err(Error::InvalidConfiguration(
            "max_samples must be greater than zero".to_owned(),
        ));
    }

    let (mut reader, sample_count) = open_iq_file(path, format)?;
    if sample_count > max_samples {
        return Err(Error::InvalidInput(format!(
            "I/Q file contains {sample_count} samples, exceeding the configured limit of {max_samples}"
        )));
    }
    reader.read_block(sample_count.max(1))
}

fn parse_iq_bytes(bytes: &[u8], format: IqFormat) -> Result<Vec<Complex32>> {
    let mut samples = Vec::with_capacity(bytes.len() / format.bytes_per_sample());
    match format {
        IqFormat::F32Le => {
            for chunk in bytes.chunks_exact(8) {
                let re = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let im = f32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                if !re.is_finite() || !im.is_finite() {
                    return Err(Error::InvalidInput(
                        "I/Q input contains a non-finite f32 sample".to_owned(),
                    ));
                }
                samples.push(Complex32::new(re, im));
            }
        }
        IqFormat::S16Le => {
            let scale = 1.0 / i16::MAX as f32;
            for chunk in bytes.chunks_exact(4) {
                let re = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 * scale;
                let im = i16::from_le_bytes([chunk[2], chunk[3]]) as f32 * scale;
                samples.push(Complex32::new(re, im));
            }
        }
    }
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_path(suffix: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("blueoxide-{nonce}-{suffix}"))
    }

    #[test]
    fn rejects_partial_complex_sample() {
        let path = temporary_path("partial.cf32");
        fs::write(&path, [0u8; 7]).unwrap();
        let error = read_iq_file(&path, IqFormat::F32Le, 10).unwrap_err();
        fs::remove_file(path).unwrap();
        assert!(error.to_string().contains("not divisible"));
    }

    #[test]
    fn rejects_non_finite_f32_sample() {
        let path = temporary_path("nan.cf32");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&f32::NAN.to_le_bytes());
        bytes.extend_from_slice(&0.0f32.to_le_bytes());
        fs::write(&path, bytes).unwrap();
        let error = read_iq_file(&path, IqFormat::F32Le, 10).unwrap_err();
        fs::remove_file(path).unwrap();
        assert!(error.to_string().contains("non-finite"));
    }

    #[test]
    fn reader_preserves_sample_indices_across_short_reads() {
        struct ShortReader {
            bytes: Vec<u8>,
            offset: usize,
        }

        impl Read for ShortReader {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                if self.offset == self.bytes.len() {
                    return Ok(0);
                }
                let count = output.len().min(3).min(self.bytes.len() - self.offset);
                output[..count].copy_from_slice(&self.bytes[self.offset..self.offset + count]);
                self.offset += count;
                Ok(count)
            }
        }

        let mut bytes = Vec::new();
        for value in [1i16, -1, 2, -2, 3, -3] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let mut reader = IqReader::new(ShortReader { bytes, offset: 0 }, IqFormat::S16Le);
        let first = reader.read_block(2).unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(reader.next_sample_index(), 2);
        let second = reader.read_block(2).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(reader.next_sample_index(), 3);
        assert!(reader.read_block(2).unwrap().is_empty());
    }
}
