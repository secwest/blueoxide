use blueoxide::advertising::decode_advertising_pdu;
use blueoxide::ble::BleChannel;
use blueoxide::demod::{Le1mDemodConfig, Le1mStreamDecoder, ReceivedAdvertisingPdu};
use blueoxide::iq::{IqFormat, open_iq_file};
use blueoxide::pcapng::{PcapNgWriter, sample_timestamp_ns};
use blueoxide::{Error, Result};
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

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

fn usage() -> &'static str {
    "blueoxide - Bluetooth/BLE SDR receive and capture tools

USAGE:
  blueoxide channels
  blueoxide decode --input FILE --channel 37|38|39 --sample-rate HZ [OPTIONS]

OPTIONS:
  --format f32le|s16le    Interleaved little-endian I/Q (default: f32le)
  --max-samples N         Maximum samples accepted from the file (default: 16000000)
  --block-samples N       Streaming decode block size (default: 262144)
  --aa-errors N           Access-address bit errors, 0..=8 (default: 1)
  --output-pcap FILE      Write CRC-valid packets as BLE PCAPNG
  --capture-start-ns N    Unix capture start in nanoseconds (default: 0)
  -h, --help              Show this help
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

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("decode") => decode(parse_decode_args(&args[1..])?),
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
