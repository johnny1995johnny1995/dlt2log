use anyhow::Result;
use clap::Parser;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input DLT file path
    input: PathBuf,

    /// Output log file path (default: same as input but with .log extension)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Verbose mode
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, PartialEq)]
enum DltVersion {
    V1,
    V2,
}

/// Helper to clean payload string by keeping only meaningful printable ASCII sequences.
fn clean_payload(buf: &[u8]) -> String {
    let mut payload = String::new();
    let mut current_segment = String::new();

    for &b in buf {
        if b >= 32 && b <= 126 {
            current_segment.push(b as char);
        } else {
            // Start of non-printable section, flush current segment if valid
            if current_segment.len() >= 2 {
                if !payload.is_empty() {
                    payload.push(' ');
                }
                payload.push_str(&current_segment);
            }
            current_segment.clear();
        }
    }
    // Flush last segment
    if current_segment.len() >= 2 {
        if !payload.is_empty() {
            payload.push(' ');
        }
        payload.push_str(&current_segment);
    }
    
    payload
}

/// Reads and parses the DLT storage header if present, returning the timestamp in microseconds.
fn read_storage_header<R: Read + Seek>(reader: &mut R, has_storage_header: bool) -> Result<u64> {
     if !has_storage_header {
        return Ok(0);
    }

    // We assume DLT\x01 format for now which is common for Vector
    let mut magic = [0u8; 4];
    if reader.read_exact(&mut magic).is_err() {
        return Ok(0); 
    }
    if &magic != b"DLT\x01" {
        return Ok(0); // Desynced or not a storage header
    }
    
    // Read seconds and microseconds/nanoseconds
    let mut sec_bytes = [0u8; 4];
    let mut extra_bytes = [0u8; 4];
    reader.read_exact(&mut sec_bytes)?;
    reader.read_exact(&mut extra_bytes)?;
    let sec = u32::from_le_bytes(sec_bytes);
    let extra = u32::from_le_bytes(extra_bytes);
    
    // Skip ECU ID in storage header (4 bytes)
    let mut ecu = [0u8; 4];
    reader.read_exact(&mut ecu)?;

    Ok((sec as u64 * 1_000_000) + (extra as u64))
}

fn parse_v1_message<R: Read + Seek>(
    reader: &mut R,
    htyp: u8,
    storage_header_timestamp: u64
) -> Result<Option<String>> {
    // Handle V1 header manually to avoid DltMessageReader buffering the whole file
    let mut rest_header = [0u8; 3]; // mcnt + len(2)
    reader.read_exact(&mut rest_header)?;
    
    let total_len = u16::from_be_bytes([rest_header[1], rest_header[2]]) as usize;
    
    if total_len < 4 {
            return Ok(None);
    }
    
    let mut msg_buf = vec![0u8; total_len];
    msg_buf[0] = htyp;
    msg_buf[1..4].copy_from_slice(&rest_header);
    
    reader.read_exact(&mut msg_buf[4..])?;
    
    // Now parse the isolated buffer
    match dlt_core::parse::dlt_message(&msg_buf[..], None, false) {
        Ok((_, parsed_msg)) => {
            if let dlt_core::parse::ParsedMessage::Item(msg) = parsed_msg {
                let mut timestamp = if storage_header_timestamp > 0 {
                    storage_header_timestamp
                } else {
                    // Fallback: Using relative timestamp from message (usually 0.1ms units in V1)
                    // We will add the base_timestamp (mtime) in main loop if needed
                    msg.header.timestamp.unwrap_or(0) as u64 * 100
                };

                // Standardize to 16 digits (microseconds)
                while timestamp > 9_999_999_999_999_999 {
                    timestamp /= 10;
                }

                let (app_id, ctx_id, log_level) = if let Some(eh) = &msg.extended_header {
                    let apid = eh.application_id.as_str().to_string();
                    let ctid = eh.context_id.as_str().to_string();
                    let level = format!("{:?}", eh.message_type);
                    let level = level.replace("Log(", "").replace(")", "").to_uppercase();
                    (apid, ctid, level)
                } else {
                    ("----".to_string(), "----".to_string(), "UNKNOWN".to_string())
                };

                use dlt_core::dlt::{PayloadContent, Value};
                let payload = match &msg.payload {
                    PayloadContent::Verbose(args) => {
                        args.iter().map(|arg| {
                            match &arg.value {
                                Value::StringVal(s) => s.clone(),
                                _ => format!("{:?}", arg.value),
                            }
                        }).collect::<Vec<_>>().join(" ")
                    }
                    _ => format!("{:?}", msg.payload),
                };

                return Ok(Some(format!("[{:016}][{} {}][{}] {}", timestamp, app_id, ctx_id, log_level, payload)));
            } 
            return Ok(None); 
        }
        Err(_) => {
            return Err(anyhow::anyhow!("Failed to parse DLT v1 message"));
        }
    }
}

fn parse_v2_message<R: Read + Seek>(
    reader: &mut R,
    htyp: u8,
    storage_header_timestamp: u64
) -> Result<Option<String>> {
    let mut htyp2_rest = [0u8; 3];
    reader.read_exact(&mut htyp2_rest)?;
    let htyp2 = (htyp as u32) | ((htyp2_rest[0] as u32) << 8) | ((htyp2_rest[1] as u32) << 16) | ((htyp2_rest[2] as u32) << 24);
    
    let mut mcnt = [0u8; 1];
    reader.read_exact(&mut mcnt)?;
    
    let mut len_bytes = [0u8; 2];
    reader.read_exact(&mut len_bytes)?;
    let total_len = u16::from_be_bytes(len_bytes) as usize; // Big Endian length
    
    let mut consumed = 7; 
    let mut v2_timestamp: u64 = 0;

    let cnti = htyp2 & 0x03;
    
    // Optional Header: MSIN + NOAR (2, if CNTI 0 or 2) + TMSP2 (9, if CNTI 0 or 1)
    if cnti == 0 || cnti == 2 {
        let mut msin_noar = [0u8; 2];
        reader.read_exact(&mut msin_noar)?;
        consumed += 2;
    }
    if cnti == 0 || cnti == 1 {
        let mut tmsp_bytes = [0u8; 8];
        reader.read_exact(&mut tmsp_bytes)?;
        let mut mystery = [0u8; 1];
        reader.read_exact(&mut mystery)?;
        v2_timestamp = u64::from_le_bytes(tmsp_bytes);
        
        // Standardize to 16 digits (microseconds)
        while v2_timestamp > 9_999_999_999_999_999 {
            v2_timestamp /= 10;
        }
        
        consumed += 9;
    }

    let timestamp = if storage_header_timestamp > 0 {
        storage_header_timestamp
    } else {
        v2_timestamp
    };

    // Bit 2: ECU ID
    if (htyp2 & 0x04) != 0 {
        let mut len = [0u8; 1];
        reader.read_exact(&mut len)?;
        let mut buf = vec![0u8; len[0] as usize];
        reader.read_exact(&mut buf)?;
        consumed += 1 + len[0] as usize;
    }
    
    let mut apid = "----".to_string();
    let mut ctid = "----".to_string();
    
    // Bit 3: AppID / CtxID
    if (htyp2 & 0x08) != 0 {
        // AppID
        let mut len = [0u8; 1];
        reader.read_exact(&mut len)?;
        let mut buf = vec![0u8; len[0] as usize];
        reader.read_exact(&mut buf)?;
        apid = String::from_utf8_lossy(&buf).to_string();
        consumed += 1 + len[0] as usize;
        
        // CtxID
        let mut len = [0u8; 1];
        reader.read_exact(&mut len)?;
        let mut buf = vec![0u8; len[0] as usize];
        reader.read_exact(&mut buf)?;
        ctid = String::from_utf8_lossy(&buf).to_string();
        consumed += 1 + len[0] as usize;
    }

    // Bit 4: Session ID
    if (htyp2 & 0x10) != 0 {
        let mut sid = [0u8; 4];
        reader.read_exact(&mut sid)?;
        consumed += 4;
    }

    // Consume remaining bytes to reach total_len
    if total_len > consumed {
        let rem = total_len - consumed;
        let mut buf = vec![0u8; rem];
        reader.read_exact(&mut buf)?;
        
        // Heuristic payload extraction
        let payload = clean_payload(&buf);
        
        return Ok(Some(format!("[{:016}][{} {}][INFO] {}", timestamp, apid, ctid, payload)));
    } else {
        return Ok(Some(format!("[{:016}][{} {}][INFO] <No Payload>", timestamp, apid, ctid)));
    }
}

fn read_v1v2_message<R: Read + Seek>(
    reader: &mut R,
    has_storage_header: bool,
    base_timestamp: u64,
) -> Result<Option<String>> {
    let storage_header_timestamp = read_storage_header(reader, has_storage_header)?;

    // Now at the start of DLT message
    let mut htyp = [0u8; 1];
    if reader.read_exact(&mut htyp).is_err() {
        return Ok(None);
    }
    
    let version = if htyp[0] == 0x35 {
        DltVersion::V1
    } else if htyp[0] == 0x4c {
        DltVersion::V2
    } else {
        return Err(anyhow::anyhow!("Unknown DLT version marker: 0x{:02x}", htyp[0]));
    };

    let result = match version {
        DltVersion::V1 => parse_v1_message(reader, htyp[0], storage_header_timestamp)?,
        DltVersion::V2 => parse_v2_message(reader, htyp[0], storage_header_timestamp)?,
    };

    if let Some(mut line) = result {
        // If the timestamp is relative (less than a threshold, e.g., 50 years in us)
        // AND we have a base_timestamp, anchor it.
        if let Some(start_idx) = line.find('[') {
            if let Some(end_idx) = line.find(']') {
                let ts_str = &line[start_idx+1..end_idx];
                if let Ok(ts_val) = ts_str.parse::<u64>() {
                    // If timestamp is small (e.g. < 10^12 us, which is ~11 days)
                    // it's almost certainly relative to boot.
                    if ts_val < 1_000_000_000_000 && base_timestamp > 0 {
                        let mut absolute_ts = base_timestamp + ts_val;
                        while absolute_ts > 9_999_999_999_999_999 {
                             absolute_ts /= 10;
                        }
                        line = format!("[{:016}]{}", absolute_ts, &line[end_idx+1..]);
                    } else {
                        let mut absolute_ts = ts_val;
                        while absolute_ts > 9_999_999_999_999_999 {
                             absolute_ts /= 10;
                        }
                        line = format!("[{:016}]{}", absolute_ts, &line[end_idx+1..]);
                    }
                }
            }
        }
        return Ok(Some(line));
    }
    
    Ok(None)
}

fn main() -> Result<()> {
    let args = Args::parse();

    let output_path = match args.output {
        Some(path) => path,
        None => {
            let mut path = args.input.clone();
            path.set_extension("log");
            path
        }
    };

    let mut input_file = File::open(&args.input)?;
    
    // Get file modification time as base for relative timestamps
    let metadata = input_file.metadata()?;
    let mtime = metadata.modified()?;
    let base_timestamp = mtime.duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;

    let mut header_magic = [0u8; 4];
    let has_storage_header = if input_file.read_exact(&mut header_magic).is_ok() {
        let found = &header_magic == b"DLT\x01";
        input_file.seek(std::io::SeekFrom::Start(0))?;
        found
    } else {
        false
    };

    let output_file = File::create(&output_path)?;
    let mut writer = BufWriter::new(output_file);
    let mut msg_count = 0;

    loop {
        let pos_before = input_file.stream_position()?;
        match read_v1v2_message(&mut input_file, has_storage_header, base_timestamp) {
            Ok(Some(line)) => {
                let pos_after = input_file.stream_position()?;
                if args.verbose {
                    println!("Message {} at offset {} (size {}): {}", msg_count, pos_before, pos_after - pos_before, line);
                }
                writer.write_all(line.as_bytes())?;
                writer.write_all(b"\n")?;
                msg_count += 1;
            }
            Ok(None) => break,
            Err(e) => {
                if args.verbose {
                    eprintln!("Error at offset {}: {}", pos_before, e);
                }
                break;
            }
        }
    }

    writer.flush()?;
    println!("Successfully processed {} messages.", msg_count);
    Ok(())
}
