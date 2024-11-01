#![feature(portable_simd)]

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use fftw::array::AlignedVec;
use fftw::plan::{C2CPlan, Sign};
use fftw::types::Flag;
use memmap2::MmapMut;
use num_complex::Complex;
use rayon::prelude::*;
use std::arch::x86_64::*;
use std::f32::consts::PI;
use std::fs::{create_dir_all, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

// Constants for headers and other settings
const HEADER_ID: u32 = 0x0000C0DE;

// Enum for SDR modes
#[derive(ValueEnum, Clone, Debug)]
enum Mode {
    Bluetooth,
    BLE,
}

// Enum for supported SDR devices
#[derive(ValueEnum, Clone, Debug)]
enum DeviceType {
    Lime,
    Xtrx,
    Bladerf,
    Hackrf,
}

// Command-line argument structure
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(value_enum)]
    mode: Mode,

    #[arg(value_enum)]
    device: DeviceType,

    #[arg(short, long, default_value_t = 2402000000)]
    center_frequency: u64,

    #[arg(short, long, default_value_t = 10000000)]
    sample_rate: u32,

    #[arg(long, default_value_t = 0)]
    frequency_offset: i32, // Frequency offset for tuning in Hz

    #[arg(short, long, default_value_t = 20)]
    num_channels: usize,

    #[arg(short, long, default_value = "output")]
    output_dir: String,

    #[arg(long, default_value = "raw_iq")]
    raw_file_prefix: String,

    #[arg(long, default_value = "channel")]
    channel_file_prefix: String,

    #[arg(long)]
    save_raw_iq: bool,

    #[arg(long, default_value_t = 2000000)]
    bandwidth: u32, // Default bandwidth for Bluetooth and BLE is 2 MHz

    #[arg(long, default_value_t = 30.0)]
    gain: f64,

    #[arg(long, default_value_t = 0.01)]
    dc_alpha: f32, // Alpha parameter for adaptive DC offset correction

    #[arg(long, default_value_t = 0.01)]
    iq_phase_error: f32,

    #[arg(long, default_value_t = 1.0)]
    iq_gain_error: f32,

    #[arg(long, default_value = "full")]
    channel_selection: String, // Either "full" for full-band or "range" for selective channels

    #[arg(long, default_value_t = 37)]
    channel_range_start: usize,

    #[arg(long, default_value_t = 39)]
    channel_range_end: usize,
}

fn main() -> Result<()> {
    let mut args = Args::parse();

    // Adjust defaults for BLE advertising channels if in BLE mode
    if args.mode == Mode::BLE {
        args.center_frequency = 2426000000; // Center across channels 37, 38, 39
        args.sample_rate = 10000000;
        args.bandwidth = 2000000;
    }

    let (tx, rx) = mpsc::sync_channel(10);
    create_dir_all(&args.output_dir)?;

    // Initialize SDR based on selected device
    let sdr: Box<dyn SDRDevice> = match args.device {
        DeviceType::Lime => Box::new(LimeSDR::new()?),
        DeviceType::Xtrx => Box::new(XTRXSdr::new()?),
        DeviceType::Bladerf => Box::new(BladeRFSDR::new()?),
        DeviceType::Hackrf => Box::new(HackRFSdr::new()?),
    };

    // Configure SDR
    sdr.set_sample_rate(args.sample_rate)?;
    sdr.set_frequency((args.center_frequency as i64 + args.frequency_offset as i64) as u64)?;
    sdr.set_rx_bandwidth(args.bandwidth as f64)?;
    sdr.set_gain(args.gain)?;

    // Data capture thread with reusable buffer
    thread::spawn(move || {
        let mut buffer = vec![Complex::new(0.0, 0.0); 8192];
        loop {
            match sdr.read_iq_samples(&mut buffer) {
                Ok(_) => tx.send(buffer.clone()).expect("Failed to send I/Q data"),
                Err(e) => eprintln!("Error reading samples: {}", e),
            }
        }
    });

    // Design polyphase filter for channelization
    let polyphase_filters = design_polyphase_filter(args.num_channels, args.sample_rate as f64, 128, args.bandwidth as f64);

    // Process I/Q samples with direct memory mapping
    for mut buffer in rx {
        if args.enable_dc_offset {
            apply_adaptive_dc_offset_correction(&mut buffer, args.dc_alpha);
        }
        if args.enable_iq_imbalance {
            apply_iq_imbalance_correction(&mut buffer, args.iq_gain_error, args.iq_phase_error);
        }

        // Apply frequency offset compensation if specified
        if args.frequency_offset != 0 {
            apply_frequency_offset(&mut buffer, args.frequency_offset, args.sample_rate);
        }

        if args.save_raw_iq {
            write_raw_iq_with_header(&buffer, &args.output_dir, &args.raw_file_prefix)?;
        }

        let channelized_output = if is_x86_feature_detected!("avx") {
            apply_polyphase_filter_avx(&buffer, &polyphase_filters, args.num_channels)
        } else {
            apply_polyphase_filter_fft(&buffer, &polyphase_filters, args.num_channels)
        };

        // Select channels for output based on selection type
        let channels = match args.channel_selection.as_str() {
            "full" => 0..args.num_channels,
            "range" => args.channel_range_start..=args.channel_range_end,
            _ => panic!("Invalid channel_selection option. Use 'full' or 'range'"),
        };

        // Write channelized output directly to memory-mapped files
        for i in channels {
            let filename = format!("{}/{}_channel_{}.iq", &args.output_dir, &args.channel_file_prefix, i);
            write_channel_to_mmap(&channelized_output[i], &filename)?;
        }
    }

    Ok(())
}

// Function for adaptive DC offset correction using an IIR high-pass filter
fn apply_adaptive_dc_offset_correction(buffer: &mut [Complex<f32>], alpha: f32) {
    let mut dc_estimate = Complex::new(0.0, 0.0);

    for sample in buffer.iter_mut() {
        dc_estimate = dc_estimate * (1.0 - alpha) + *sample * alpha;
        *sample -= dc_estimate;
    }
}

// Function for IQ imbalance correction (AVX-optimized if available)
fn apply_iq_imbalance_correction(buffer: &mut [Complex<f32>], gain_error: f32, phase_error: f32) {
    let cos_phase = phase_error.cos();
    let sin_phase = phase_error.sin();

    if is_x86_feature_detected!("avx") {
        let cos_avx = unsafe { _mm256_set1_ps(cos_phase) };
        let sin_avx = unsafe { _mm256_set1_ps(sin_phase) };
        let gain_avx = unsafe { _mm256_set1_ps(gain_error) };

        for chunk in buffer.chunks_mut(8) {
            let i_vals = unsafe { _mm256_loadu_ps(chunk.iter().map(|s| s.re).collect::<Vec<f32>>().as_ptr()) };
            let q_vals = unsafe { _mm256_loadu_ps(chunk.iter().map(|s| s.im).collect::<Vec<f32>>().as_ptr()) };

            let corrected_i = unsafe { _mm256_sub_ps(_mm256_mul_ps(i_vals, cos_avx), _mm256_mul_ps(q_vals, _mm256_mul_ps(gain_avx, sin_avx))) };
            let corrected_q = unsafe { _mm256_add_ps(_mm256_mul_ps(q_vals, cos_avx), _mm256_mul_ps(i_vals, _mm256_mul_ps(gain_avx, sin_avx))) };

            for i in 0..chunk.len() {
                chunk[i].re = unsafe { _mm256_get_ps(corrected_i, i) };
                chunk[i].im = unsafe { _mm256_get_ps(corrected_q, i) };
            }
        }
    } else {
        for sample in buffer.iter_mut() {
            let i = sample.re * cos_phase - sample.im * gain_error * sin_phase;
            let q = sample.im * cos_phase + sample.re * gain_error * sin_phase;
            sample.re = i;
            sample.im = q;
        }
    }
}

// Function to apply a frequency offset
fn apply_frequency_offset(buffer: &mut [Complex<f32>], freq_offset: i32, sample_rate: u32) {
    let freq_rad = 2.0 * PI * (freq_offset as f32) / (sample_rate as f32);
    for (i, sample) in buffer.iter_mut().enumerate() {
        let angle = freq_rad * i as f32;
        let cos_offset = angle.cos();
        let sin_offset = angle.sin();
        let adjusted = Complex::new(
            sample.re * cos_offset - sample.im * sin_offset,
            sample.im * cos_offset + sample.re * sin_offset,
        );
        *sample = adjusted;
    }
}

// Polyphase filter design
fn design_polyphase_filter(num_channels: usize, sample_rate: f64, filter_length: usize, bandwidth_per_channel: f64) -> Vec<Vec<f32>> {
    let nyquist = sample_rate / 2.0;
    let cutoff = bandwidth_per_channel / nyquist;
    let taps = (0..filter_length)
        .map(|n| {
            let x = n as f64 - (filter_length as f64) / 2.0;
            if x.abs() < std::f64::EPSILON {
                2.0 * cutoff
            } else {
                (2.0 * cutoff * x.sin()) / (std::f64::consts::PI * x)
            }
        })
        .collect::<Vec<_>>();
    taps.chunks(num_channels).map(|chunk| chunk.to_vec()).collect()
}

// AVX-optimized polyphase filter
fn apply_polyphase_filter_avx(buffer: &[Complex<f32>], polyphase_filters: &[Vec<f32>], num_channels: usize) -> Vec<Vec<f32>> {
    let chunk_size = 8;
    let channel_outputs: Vec<Vec<f32>> = (0..num_channels).into_par_iter().map(|channel_idx| {
        let mut channel_output = vec![0.0; buffer.len() / num_channels];
        let filter = &polyphase_filters[channel_idx];
        let filter_avx = unsafe { _mm256_loadu_ps(&filter[0]) };

        for (i, chunk) in buffer.chunks(num_channels * chunk_size).enumerate() {
            let mut output_vals = [0.0f32; chunk_size];

            for j in 0..chunk_size {
                let sample_avx = unsafe { _mm256_loadu_ps(&chunk[j].re) };
                let filtered_sample = unsafe { _mm256_mul_ps(sample_avx, filter_avx) };
                output_vals[j] = unsafe { _mm256_reduce_add_ps(filtered_sample) };
            }
            channel_output[i] = output_vals[0];
        }
        channel_output
    }).collect();

    channel_outputs
}

// FFT-based polyphase filter for non-AVX fallback
fn apply_polyphase_filter_fft(buffer: &[Complex<f32>], polyphase_filters: &[Vec<f32>], num_channels: usize) -> Vec<Vec<f32>> {
    let fft_size = buffer.len() / num_channels;
    (0..num_channels).into_par_iter().map(|channel_idx| {
        let filter = &polyphase_filters[channel_idx];
        let mut input = AlignedVec::new(fft_size);
        let mut output = AlignedVec::new(fft_size);
        let plan = C2CPlan::aligned(&[fft_size], Sign::Forward, Flag::MEASURE).expect("FFTW plan failed");

        for (i, &sample) in buffer.iter().step_by(num_channels).enumerate() {
            input[i] = Complex::new(sample.re * filter[i % filter.len()], sample.im * filter[i % filter.len()]);
        }

        plan.c2c(&mut input, &mut output).expect("FFTW execution failed");

        output.iter().map(|sample| sample.re).collect()
    }).collect()
}

// Write raw I/Q samples with header
fn write_raw_iq_with_header(buffer: &[Complex<f32>], output_dir: &str, file_prefix: &str) -> Result<()> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as u64;
    let filename = format!("{}/{}_{}.iq", output_dir, file_prefix, timestamp);
    let mut file = BufWriter::new(File::create(&filename).context("Failed to create raw I/Q file")?);

    let header = [
        HEADER_ID.to_be_bytes(),
        (buffer.len() as u32).to_be_bytes(),
        timestamp.to_be_bytes(),
    ]
    .concat();

    file.write_all(&header).context("Failed to write header")?;
    for sample in buffer {
        file.write_all(&sample.re.to_be_bytes())?;
        file.write_all(&sample.im.to_be_bytes())?;
    }

    Ok(())
}

// Write channel output directly to memory-mapped file
fn write_channel_to_mmap(samples: &[f32], filename: &str) -> Result<()> {
    let file = OpenOptions::new().read(true).write(true).create(true).open(filename)?;
    file.set_len((samples.len() * std::mem::size_of::<f32>()) as u64)?;
    let mut mmap = unsafe { MmapMut::map_mut(&file)? };

    for (i, &sample) in samples.iter().enumerate() {
        let bytes = sample.to_ne_bytes();
        mmap[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
    }

    mmap.flush()?;
    Ok(())
}
