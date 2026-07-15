use blueoxide::advertising::decode_advertising_pdu;
use blueoxide::backends::bladerf::{BladeRfOptions, BladeRfSource};
use blueoxide::backends::limesdr::{LimeSdrOptions, LimeSdrSource};
use blueoxide::backends::xtrx::{XtrxOptions, XtrxSource};
use blueoxide::ble::BleChannel;
use blueoxide::capture::{
    CaptureLimits, CaptureStats, CapturedAdvertisingPdu, capture_primary_advertising,
};
use blueoxide::demod::{Le1mDemodConfig, Le1mStreamDecoder, ReceivedAdvertisingPdu};
use blueoxide::iq::{IqFormat, open_iq_file};
use blueoxide::pcapng::{PcapNgWriter, sample_timestamp_ns};
use blueoxide::sdr::{IqSource, SdrConfig};
use blueoxide::{Error, Result};
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_MAX_SAMPLES: usize = 16_000_000;
const DEFAULT_BLOCK_SAMPLES: usize = 262_144;

#[derive(Debug)]
struct DecodeArgs {
    input: PathBuf,
    format: IqFormat,
    channel: BleChannel,
    sample_rate_hz: u32,
    max_samples: usize,
    block_samples: usize,
    max_access_address_errors: u8,
    output_pcap: Option<PathBuf>,
    capture_start_ns: u64,
}

#[derive(Debug)]
struct CaptureArgs {
    device: String,
    identifier: Option<String>,
    channel: BleChannel,
    sample_rate_hz: u32,
    bandwidth_hz: u32,
    gain_db: f32,
    rx_channel: u8,
    duration: Duration,
    block_samples: usize,
    read_timeout_ms: u64,
    max_access_address_errors: u8,
    output_pcap: Option<PathBuf>,
    capture_start_ns: Option<u64>,
}

fn usage() -> &'static str {
    "blueoxide - Bluetooth/BLE SDR receive and capture tools

USAGE:
  blueoxide channels
  blueoxide backends
  blueoxide decode --input FILE --channel 37|38|39 --sample-rate HZ [OPTIONS]
  blueoxide capture --device bladerf|limesdr|xtrx --channel 37|38|39 [OPTIONS]

DECODE OPTIONS:
  --format f32le|s16le    Interleaved little-endian I/Q (default: f32le)
  --max-samples N         Maximum samples accepted from the file (default: 16000000)
  --block-samples N       Streaming decode block size (default: 262144)
  --aa-errors N           Access-address bit errors, 0..=8 (default: 1)
  --output-pcap FILE      Write CRC-valid packets as BLE PCAPNG
  --capture-start-ns N    Unix capture start in nanoseconds (default: 0)
  -h, --help              Show this help

CAPTURE OPTIONS:
  --identifier STRING     Native backend device identifier
  --sample-rate HZ        Complex sample rate (default: 4000000)
  --bandwidth HZ          RX bandwidth (default: 2000000)
  --gain DB               RX gain in dB (default: 30)
  --rx-channel N          Hardware RX channel (default: 0)
  --seconds N             Capture duration (default: 10)
  --block-samples N       Native read size (default: 8192)
  --read-timeout-ms N     Native read timeout (default: 1000)
  --aa-errors N           Access-address bit errors, 0..=8 (default: 1)
  --output-pcap FILE      Write CRC-valid packets as BLE PCAPNG
  --capture-start-ns N    Override Unix capture start in nanoseconds
"
}

fn value_after(args: &[String], index: &mut usize, option: &str) -> Result<String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| Error::InvalidConfiguration(format!("missing value after {option}")))
}

fn parse_number<T: std::str::FromStr>(value: &str, option: &str) -> Result<T> {
    value
        .parse()
        .map_err(|_| Error::InvalidConfiguration(format!("invalid value {value:?} for {option}")))
}

fn parse_decode_args(args: &[String]) -> Result<DecodeArgs> {
    let mut input = None;
    let mut format = IqFormat::F32Le;
    let mut channel = None;
    let mut sample_rate_hz = None;
    let mut max_samples = DEFAULT_MAX_SAMPLES;
    let mut block_samples = DEFAULT_BLOCK_SAMPLES;
    let mut max_access_address_errors = 1u8;
    let mut output_pcap = None;
    let mut capture_start_ns = 0u64;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--input" => input = Some(PathBuf::from(value_after(args, &mut index, "--input")?)),
            "--format" => format = IqFormat::parse(&value_after(args, &mut index, "--format")?)?,
            "--channel" => {
                let value = value_after(args, &mut index, "--channel")?;
                channel = Some(BleChannel::new(parse_number(&value, "--channel")?)?);
            }
            "--sample-rate" => {
                let value = value_after(args, &mut index, "--sample-rate")?;
                sample_rate_hz = Some(parse_number(&value, "--sample-rate")?);
            }
            "--max-samples" => {
                let value = value_after(args, &mut index, "--max-samples")?;
                max_samples = parse_number(&value, "--max-samples")?;
            }
            "--block-samples" => {
                let value = value_after(args, &mut index, "--block-samples")?;
                block_samples = parse_number(&value, "--block-samples")?;
            }
            "--aa-errors" => {
                let value = value_after(args, &mut index, "--aa-errors")?;
                max_access_address_errors = parse_number(&value, "--aa-errors")?;
            }
            "--output-pcap" => {
                output_pcap = Some(PathBuf::from(value_after(
                    args,
                    &mut index,
                    "--output-pcap",
                )?));
            }
            "--capture-start-ns" => {
                let value = value_after(args, &mut index, "--capture-start-ns")?;
                capture_start_ns = parse_number(&value, "--capture-start-ns")?;
            }
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown decode option {unknown:?}"
                )));
            }
        }
        index += 1;
    }

    Ok(DecodeArgs {
        input: input.ok_or_else(|| {
            Error::InvalidConfiguration("decode requires --input FILE".to_owned())
        })?,
        format,
        channel: channel.ok_or_else(|| {
            Error::InvalidConfiguration("decode requires --channel 37|38|39".to_owned())
        })?,
        sample_rate_hz: sample_rate_hz.ok_or_else(|| {
            Error::InvalidConfiguration("decode requires --sample-rate HZ".to_owned())
        })?,
        max_samples,
        block_samples,
        max_access_address_errors,
        output_pcap,
        capture_start_ns,
    })
}

fn parse_capture_args(args: &[String]) -> Result<CaptureArgs> {
    let mut device = None;
    let mut identifier = None;
    let mut channel = None;
    let mut sample_rate_hz = 4_000_000u32;
    let mut bandwidth_hz = 2_000_000u32;
    let mut gain_db = 30.0f32;
    let mut rx_channel = 0u8;
    let mut duration = Duration::from_secs(10);
    let mut block_samples = 8_192usize;
    let mut read_timeout_ms = 1_000u64;
    let mut max_access_address_errors = 1u8;
    let mut output_pcap = None;
    let mut capture_start_ns = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--device" => device = Some(value_after(args, &mut index, "--device")?),
            "--identifier" => identifier = Some(value_after(args, &mut index, "--identifier")?),
            "--channel" => {
                let value = value_after(args, &mut index, "--channel")?;
                channel = Some(BleChannel::new(parse_number(&value, "--channel")?)?);
            }
            "--sample-rate" => {
                let value = value_after(args, &mut index, "--sample-rate")?;
                sample_rate_hz = parse_number(&value, "--sample-rate")?;
            }
            "--bandwidth" => {
                let value = value_after(args, &mut index, "--bandwidth")?;
                bandwidth_hz = parse_number(&value, "--bandwidth")?;
            }
            "--gain" => {
                let value = value_after(args, &mut index, "--gain")?;
                gain_db = parse_number(&value, "--gain")?;
            }
            "--rx-channel" => {
                let value = value_after(args, &mut index, "--rx-channel")?;
                rx_channel = parse_number(&value, "--rx-channel")?;
            }
            "--seconds" => {
                let value = value_after(args, &mut index, "--seconds")?;
                let seconds: f64 = parse_number(&value, "--seconds")?;
                duration = Duration::try_from_secs_f64(seconds).map_err(|_| {
                    Error::InvalidConfiguration(
                        "--seconds must be finite, greater than zero, and representable as a duration"
                            .to_owned(),
                    )
                })?;
                if duration == Duration::ZERO {
                    return Err(Error::InvalidConfiguration(
                        "--seconds must be greater than zero".to_owned(),
                    ));
                }
            }
            "--block-samples" => {
                let value = value_after(args, &mut index, "--block-samples")?;
                block_samples = parse_number(&value, "--block-samples")?;
            }
            "--read-timeout-ms" => {
                let value = value_after(args, &mut index, "--read-timeout-ms")?;
                read_timeout_ms = parse_number(&value, "--read-timeout-ms")?;
            }
            "--aa-errors" => {
                let value = value_after(args, &mut index, "--aa-errors")?;
                max_access_address_errors = parse_number(&value, "--aa-errors")?;
            }
            "--output-pcap" => {
                output_pcap = Some(PathBuf::from(value_after(
                    args,
                    &mut index,
                    "--output-pcap",
                )?));
            }
            "--capture-start-ns" => {
                let value = value_after(args, &mut index, "--capture-start-ns")?;
                capture_start_ns = Some(parse_number(&value, "--capture-start-ns")?);
            }
            "-h" | "--help" => {
                print!("{}", usage());
                std::process::exit(0);
            }
            unknown => {
                return Err(Error::InvalidConfiguration(format!(
                    "unknown capture option {unknown:?}"
                )));
            }
        }
        index += 1;
    }

    if block_samples == 0 || block_samples > c_int_max_as_usize() {
        return Err(Error::InvalidConfiguration(
            "--block-samples must be in 1..=2147483647 for current live backends".to_owned(),
        ));
    }
    if read_timeout_ms == 0 || read_timeout_ms > u32::MAX as u64 {
        return Err(Error::InvalidConfiguration(
            "--read-timeout-ms must be in 1..=4294967295 for current live backends".to_owned(),
        ));
    }
    if !gain_db.is_finite() {
        return Err(Error::InvalidConfiguration(
            "--gain must be finite".to_owned(),
        ));
    }
    Ok(CaptureArgs {
        device: device.ok_or_else(|| {
            Error::InvalidConfiguration("capture requires --device bladerf|limesdr|xtrx".to_owned())
        })?,
        identifier,
        channel: channel.ok_or_else(|| {
            Error::InvalidConfiguration("capture requires --channel 37|38|39".to_owned())
        })?,
        sample_rate_hz,
        bandwidth_hz,
        gain_db,
        rx_channel,
        duration,
        block_samples,
        read_timeout_ms,
        max_access_address_errors,
        output_pcap,
        capture_start_ns,
    })
}

const fn c_int_max_as_usize() -> usize {
    i32::MAX as usize
}

fn print_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn print_packet(packet: &ReceivedAdvertisingPdu) {
    let semantic = decode_advertising_pdu(&packet.pdu)
        .map(|decoded| decoded.to_string())
        .unwrap_or_else(|error| format!("decode_error={error}"));
    println!(
        "channel={} sample={} phase={} inverted={} aa_errors={} pdu_type={} carrier_offset_hz={:.1} deviation_hz={:.1} header={} payload={} crc={} semantic=\"{}\"",
        packet.pdu.channel.index(),
        packet.access_address_sample,
        packet.symbol_phase,
        packet.pdu.inverted,
        packet.pdu.access_address_errors,
        packet.pdu.pdu_type(),
        packet.estimated_carrier_offset_hz,
        packet.estimated_deviation_hz,
        print_hex(&packet.pdu.header),
        print_hex(&packet.pdu.payload),
        print_hex(&packet.pdu.crc),
        semantic.replace('"', "'"),
    );
}

fn decode(args: DecodeArgs) -> Result<()> {
    if args.block_samples == 0 {
        return Err(Error::InvalidConfiguration(
            "--block-samples must be greater than zero".to_owned(),
        ));
    }
    let (mut reader, sample_count) = open_iq_file(&args.input, args.format)?;
    if sample_count > args.max_samples {
        return Err(Error::InvalidInput(format!(
            "I/Q file contains {sample_count} samples, exceeding the configured limit of {}",
            args.max_samples
        )));
    }

    let config = Le1mDemodConfig {
        sample_rate_hz: args.sample_rate_hz,
        max_access_address_errors: args.max_access_address_errors,
    };
    let mut decoder = Le1mStreamDecoder::new(args.channel, config)?;
    let mut pcap = match &args.output_pcap {
        Some(path) => Some(PcapNgWriter::new(BufWriter::new(File::create(path)?))?),
        None => None,
    };
    let mut packet_count = 0usize;

    loop {
        let first_sample = reader.next_sample_index();
        let samples = reader.read_block(args.block_samples)?;
        if samples.is_empty() {
            break;
        }
        let batch = decoder.push(first_sample, &samples)?;
        if let Some(discontinuity) = batch.discontinuity {
            eprintln!(
                "sample discontinuity: expected {}, observed {}",
                discontinuity.expected_first_sample, discontinuity.observed_first_sample
            );
        }
        for packet in &batch.packets {
            print_packet(packet);
            if let Some(writer) = &mut pcap {
                let timestamp = sample_timestamp_ns(
                    args.capture_start_ns,
                    packet.access_address_sample,
                    args.sample_rate_hz,
                )?;
                writer.write_advertising(packet, timestamp)?;
            }
        }
        packet_count += batch.packets.len();
    }

    if let Some(writer) = pcap {
        writer.into_inner().flush()?;
    }
    eprintln!("decoded {packet_count} CRC-valid packet(s) from {sample_count} sample(s)");
    Ok(())
}

fn current_unix_time_ns() -> Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            Error::InvalidInput(format!("system clock precedes Unix epoch: {error}"))
        })?;
    u64::try_from(duration.as_nanos())
        .map_err(|_| Error::InvalidInput("current Unix timestamp exceeds u64".to_owned()))
}

fn capture(args: CaptureArgs) -> Result<()> {
    let device = args.device.to_ascii_lowercase();
    if !matches!(device.as_str(), "bladerf" | "limesdr" | "lime" | "xtrx") {
        return Err(Error::InvalidConfiguration(format!(
            "capture device {:?} is not implemented; currently available: bladerf, limesdr, xtrx",
            args.device
        )));
    }
    if !args.channel.is_primary_advertising() {
        return Err(Error::InvalidConfiguration(format!(
            "capture currently requires BLE advertising channel 37, 38, or 39; got {}",
            args.channel.index()
        )));
    }
    let demod_config = Le1mDemodConfig {
        sample_rate_hz: args.sample_rate_hz,
        max_access_address_errors: args.max_access_address_errors,
    };
    demod_config.validate()?;
    let radio_config = SdrConfig {
        center_frequency_hz: args.channel.center_frequency_hz(),
        sample_rate_hz: args.sample_rate_hz,
        bandwidth_hz: args.bandwidth_hz,
        gain_db: args.gain_db,
        channel: args.rx_channel,
    };

    let stats = match device.as_str() {
        "bladerf" => {
            let mut source =
                BladeRfSource::open(args.identifier.as_deref(), BladeRfOptions::default())?;
            let stats = capture_from_source(&mut source, &args, &radio_config, demod_config)?;
            if let Some(applied) = source.applied_config() {
                eprintln!(
                    "bladeRF applied sample_rate={} bandwidth={}",
                    applied.sample_rate_hz, applied.bandwidth_hz
                );
            }
            stats
        }
        "limesdr" | "lime" => {
            let mut source =
                LimeSdrSource::open(args.identifier.as_deref(), LimeSdrOptions::default())?;
            let stats = capture_from_source(&mut source, &args, &radio_config, demod_config)?;
            if let Some(applied) = source.applied_config() {
                eprintln!(
                    "LimeSDR applied sample_rate={} bandwidth={}",
                    applied.sample_rate_hz, applied.bandwidth_hz
                );
            }
            stats
        }
        "xtrx" => {
            let mut source = XtrxSource::open(args.identifier.as_deref(), XtrxOptions::default())?;
            let stats = capture_from_source(&mut source, &args, &radio_config, demod_config)?;
            if let Some(applied) = source.applied_config() {
                eprintln!(
                    "XTRX applied sample_rate={} bandwidth={}",
                    applied.sample_rate_hz, applied.bandwidth_hz
                );
            }
            stats
        }
        _ => unreachable!(),
    };
    eprintln!(
        "capture complete: samples={} packets={} overruns={} dropped={} discontinuities={}",
        stats.samples_received,
        stats.packets_decoded,
        stats.overruns,
        stats.dropped_samples,
        stats.discontinuities
    );
    Ok(())
}

fn capture_from_source<S: IqSource>(
    source: &mut S,
    args: &CaptureArgs,
    radio_config: &SdrConfig,
    demod_config: Le1mDemodConfig,
) -> Result<CaptureStats> {
    let capture_start_ns = args
        .capture_start_ns
        .map(Ok)
        .unwrap_or_else(current_unix_time_ns)?;
    let mut pcap = match &args.output_pcap {
        Some(path) => Some(PcapNgWriter::new(BufWriter::new(File::create(path)?))?),
        None => None,
    };

    let stats = capture_primary_advertising(
        source,
        radio_config,
        args.channel,
        demod_config,
        CaptureLimits {
            maximum_samples: None,
            maximum_duration: Some(args.duration),
            read_timeout: Duration::from_millis(args.read_timeout_ms),
            block_samples: args.block_samples,
        },
        |captured: &CapturedAdvertisingPdu| {
            print_packet(&captured.observation);
            if let Some(writer) = &mut pcap {
                let timestamp = sample_timestamp_ns(
                    capture_start_ns,
                    captured.relative_sample_index,
                    args.sample_rate_hz,
                )?;
                writer.write_advertising(&captured.observation, timestamp)?;
            }
            Ok(())
        },
    )?;
    if let Some(writer) = pcap {
        writer.into_inner().flush()?;
    }
    Ok(stats)
}

fn backends() {
    match BladeRfSource::probe_library() {
        Ok(library) => println!("bladerf  library available: {library}"),
        Err(error) => println!("bladerf  unavailable: {error}"),
    }
    match LimeSdrSource::probe_library() {
        Ok(library) => println!("limesdr  library available: {library}"),
        Err(error) => println!("limesdr  unavailable: {error}"),
    }
    match XtrxSource::probe_library() {
        Ok(library) => println!("xtrx     library available: {library}"),
        Err(error) => println!("xtrx     unavailable: {error}"),
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("decode") => decode(parse_decode_args(&args[1..])?),
        Some("capture") => capture(parse_capture_args(&args[1..])?),
        Some("backends") => {
            backends();
            Ok(())
        }
        Some("channels") => {
            for index in 0..=39 {
                let channel = BleChannel::new(index)?;
                println!(
                    "{:>2}  {} Hz{}",
                    index,
                    channel.center_frequency_hz(),
                    if channel.is_primary_advertising() {
                        "  primary advertising"
                    } else {
                        ""
                    }
                );
            }
            Ok(())
        }
        Some("-h" | "--help") | None => {
            print!("{}", usage());
            Ok(())
        }
        Some(command) => Err(Error::InvalidConfiguration(format!(
            "unknown command {command:?}\n\n{}",
            usage()
        ))),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("blueoxide: {error}");
        std::process::exit(2);
    }
}
