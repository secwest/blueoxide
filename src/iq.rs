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
    let sample_count = length / bytes_per_sample;
    if sample_count > max_samples {
        return Err(Error::InvalidInput(format!(
            "I/Q file contains {sample_count} samples, exceeding the configured limit of {max_samples}"
        )));
    }

    let mut reader = BufReader::new(file);
    let mut bytes = vec![0u8; length];
    reader.read_exact(&mut bytes)?;
    let mut samples = Vec::with_capacity(sample_count);

    match format {
        IqFormat::F32Le => {
            for chunk in bytes.chunks_exact(8) {
                let re = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
                let im = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
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
                let re = i16::from_le_bytes(chunk[0..2].try_into().unwrap()) as f32 * scale;
                let im = i16::from_le_bytes(chunk[2..4].try_into().unwrap()) as f32 * scale;
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
}
