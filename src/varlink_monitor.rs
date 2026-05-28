//! varlink_monitor.rs
//!
//! Opens a synchronous varlink connection to the `io.systemd.Resolve.Monitor` socket provided
//! by systemd-resolved and calls `SubscribeQueryResults` with `more:true` so the server pushes
//! updates for the entire lifetime of the connection.
//!
//! Wire protocol (NUL-terminated JSON frames over a Unix stream socket):
//!
//!   → {"method":"io.systemd.Resolve.Monitor.SubscribeQueryResults","parameters":{},"more":true}\0
//!   ← {"parameters":{"ready":true},"continues":true}\0
//!   ← {"parameters":{<QueryResult>},"continues":true}\0
//!   ← …
//!
//! QueryResult:
//!   state    – "success" | "failure" | …
//!   question – [{name, type, class}]  (may be null on older systemd builds)
//!   answer   – [{rr:{key:{name,type,class}, address:[…]}, …}]
//!

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::unix::net::UnixStream;

use anyhow::{anyhow, bail};
use log::error;
use serde::Deserialize;

/// Top-level varlink message envelope.
#[derive(Deserialize, Debug)]
struct Envelope {
    parameters: Option<Payload>,
    /// Server sets this to `true` to signal more frames will follow.
    #[serde(default)]
    continues: bool,
    /// Set on server-side error replies.
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Payload {
    QueryResult { state: QueryResultState, question: Vec<Question>, answer: Vec<RrEntry> },
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum QueryResultState {
    Success,
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Debug)]
struct Question {
    name: String,
    // #[serde(rename = "type")]
    // ty: String,
    // class: String,
}

/// One resource-record entry inside the `answer` array.
#[derive(Deserialize, Debug)]
struct RrEntry {
    rr: RrData,
}

#[derive(Deserialize, Debug)]
struct RrData {
    key: RrKey,
    /// Raw address bytes: 4 = IPv4, 16 = IPv6.
    address: Vec<u8>,
}

#[derive(Deserialize, Debug)]
struct RrKey {
    // name: Option<String>,
    #[serde(rename = "type")]
    ty: u32,
}

/// DNS RR type codes we care about.
const RR_A: u32 = 1;
const RR_AAAA: u32 = 28;

/// Connect to `socket_path`, subscribe to query results, and call `callback`
/// for every successful DNS response that carries A or AAAA records.
///
/// Blocks indefinitely; returns only on I/O error or a clean server close.
pub fn subscribe<F>(mut stream: UnixStream, mut callback: F) -> anyhow::Result<()>
where
    F: FnMut(String, Vec<Ipv4Addr>, Vec<Ipv6Addr>),
{
    // Send the monitor call with `more:true` (varlink streaming mode).
    stream.write_all("{\"method\":\"io.systemd.Resolve.Monitor.SubscribeQueryResults\",\"parameters\":{},\"more\":true}\0".as_bytes())?;

    let reader = BufReader::new(stream);

    // Read one NUL-delimited JSON frame at a time.
    for frame in reader.split(0u8) {
        let bytes = frame?;
        if bytes.is_empty() {
            continue;
        }
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        let Ok(envelope) = serde_json::from_str::<Envelope>(text) else {
            continue;
        };
        if let Some(err) = envelope.error.as_ref() {
            bail!("varlink server error: {}", err);
        }
        let Some(params) = envelope.parameters else {
            continue;
        };

        match params {
            Payload::QueryResult { state, question, answer } => {
                if let Err(e) = handle_result(state, question, answer, &mut callback) {
                    error!("varlink error: {}", e);
                }
            }
        }
        if !envelope.continues {
            break;
        }
    }

    Ok(())
}

fn handle_result<F>(
    state: QueryResultState,
    question: Vec<Question>,
    answer: Vec<RrEntry>,
    callback: &mut F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(String, Vec<Ipv4Addr>, Vec<Ipv6Addr>),
{
    // Ignore failed queries.
    if !matches!(state, QueryResultState::Success) {
        return Ok(());
    }
    let Some(hostname) = question.into_iter().next().map(|v| v.name) else {
        return Ok(());
    };

    let mut addrs_v4: Vec<Ipv4Addr> = Vec::new();
    let mut addrs_v6: Vec<Ipv6Addr> = Vec::new();

    for entry in &answer {
        let address = &entry.rr.address;
        match entry.rr.key.ty {
            RR_A if address.len() == 4 => {
                let octets: [u8; 4] =
                    address.get(..4).ok_or_else(|| anyhow!("invalid ipv4 address"))?.try_into()?;
                addrs_v4.push(octets.into());
            }
            RR_AAAA if address.len() == 16 => {
                let octets: [u8; 16] =
                    address.get(..16).ok_or_else(|| anyhow!("invalid ipv6 address"))?.try_into()?;
                addrs_v6.push(octets.into());
            }
            _ => {}
        }
    }

    if addrs_v4.is_empty() && addrs_v6.is_empty() {
        return Ok(());
    }

    callback(hostname, addrs_v4, addrs_v6);
    Ok(())
}
