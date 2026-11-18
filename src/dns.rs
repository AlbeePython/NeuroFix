use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolver {
    pub id: usize,
    pub name: String,
    pub primary: Ipv4Addr,
    pub secondary: Option<Ipv4Addr>,
    pub custom: bool,
}

impl Resolver {
    pub fn servers(&self) -> Vec<Ipv4Addr> {
        match self.secondary {
            Some(secondary) => vec![self.primary, secondary],
            None => vec![self.primary],
        }
    }
}

pub struct Service {
    pub name: &'static str,
    pub host: &'static str,
}

pub const SERVICES: [Service; 3] = [
    Service {
        name: "Gemini",
        host: "gemini.google.com",
    },
    Service {
        name: "Claude",
        host: "claude.ai",
    },
    Service {
        name: "ChatGPT",
        host: "chatgpt.com",
    },
];

type Entry = (&'static str, [u8; 4], Option<[u8; 4]>);

pub fn builtin() -> Vec<Resolver> {
    let entries: [Entry; 6] = [
        ("Cloudflare", [1, 1, 1, 1], Some([1, 0, 0, 1])),
        ("Google Public DNS", [8, 8, 8, 8], Some([8, 8, 4, 4])),
        ("Quad9", [9, 9, 9, 9], Some([149, 112, 112, 112])),
        ("OpenDNS", [208, 67, 222, 222], Some([208, 67, 220, 220])),
        ("AdGuard DNS", [94, 140, 14, 14], Some([94, 140, 15, 15])),
        ("Yandex DNS", [77, 88, 8, 8], Some([77, 88, 8, 1])),
    ];

    entries
        .iter()
        .enumerate()
        .map(|(index, (name, primary, secondary))| Resolver {
            id: index,
            name: (*name).to_owned(),
            primary: Ipv4Addr::from(*primary),
            secondary: secondary.as_ref().map(|ip| Ipv4Addr::from(*ip)),
            custom: false,
        })
        .collect()
}

pub fn new_id(existing: &[Resolver]) -> usize {
    existing.iter().map(|r| r.id + 1).max().unwrap_or(0)
}

pub fn query_a(
    server: Ipv4Addr,
    host: &str,
    timeout: Duration,
) -> Option<(Duration, Vec<Ipv4Addr>)> {
    let id = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0)
        & 0xFFFF) as u16;
    let packet = build_query(id, host)?;

    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.set_read_timeout(Some(timeout)).ok()?;
    socket.connect(SocketAddr::from((server, 53))).ok()?;

    let started = Instant::now();
    socket.send(&packet).ok()?;

    let mut buffer = [0u8; 1500];
    let size = socket.recv(&mut buffer).ok()?;
    let elapsed = started.elapsed();

    if size < 12 {
        return None;
    }
    let response_id = u16::from_be_bytes([buffer[0], buffer[1]]);
    if response_id != id {
        return None;
    }
    let flags = u16::from_be_bytes([buffer[2], buffer[3]]);
    if flags & 0x8000 == 0 || (flags & 0x000F) != 0 {
        return None;
    }

    let addresses = parse_answers(&buffer[..size])?;
    if addresses.is_empty() {
        None
    } else {
        Some((elapsed, addresses))
    }
}

pub fn latency(server: Ipv4Addr, samples: usize) -> Option<Duration> {
    let host = "example.com";
    let mut values = Vec::with_capacity(samples);
    for _ in 0..samples {
        if let Some((elapsed, _)) = query_a(server, host, Duration::from_millis(1200)) {
            values.push(elapsed);
        }
    }
    if values.is_empty() {
        return None;
    }
    values.sort();
    Some(values[values.len() / 2])
}

fn build_query(id: u16, host: &str) -> Option<Vec<u8>> {
    let mut packet = Vec::with_capacity(64);
    packet.extend_from_slice(&id.to_be_bytes());
    packet.extend_from_slice(&0x0100u16.to_be_bytes());
    packet.extend_from_slice(&1u16.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes());

    for label in host.split('.') {
        if label.is_empty() || label.len() > 63 {
            return None;
        }
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0);
    packet.extend_from_slice(&1u16.to_be_bytes());
    packet.extend_from_slice(&1u16.to_be_bytes());

    Some(packet)
}

fn parse_answers(buffer: &[u8]) -> Option<Vec<Ipv4Addr>> {
    if buffer.len() < 12 {
        return None;
    }
    let questions = u16::from_be_bytes([buffer[4], buffer[5]]) as usize;
    let answers = u16::from_be_bytes([buffer[6], buffer[7]]) as usize;

    let mut offset = 12;
    for _ in 0..questions {
        offset = skip_name(buffer, offset)?;
        offset = offset.checked_add(4)?;
        if offset > buffer.len() {
            return None;
        }
    }

    let mut addresses = Vec::new();
    for _ in 0..answers {
        offset = skip_name(buffer, offset)?;
        if offset + 10 > buffer.len() {
            return None;
        }
        let record_type = u16::from_be_bytes([buffer[offset], buffer[offset + 1]]);
        let data_len = u16::from_be_bytes([buffer[offset + 8], buffer[offset + 9]]) as usize;
        offset += 10;
        if offset + data_len > buffer.len() {
            return None;
        }
        if record_type == 1 && data_len == 4 {
            addresses.push(Ipv4Addr::new(
                buffer[offset],
                buffer[offset + 1],
                buffer[offset + 2],
                buffer[offset + 3],
            ));
        }
        offset += data_len;
    }

    Some(addresses)
}

fn skip_name(buffer: &[u8], mut offset: usize) -> Option<usize> {
    loop {
        let length = *buffer.get(offset)? as usize;
        if length == 0 {
            return Some(offset + 1);
        }
        if length & 0xC0 == 0xC0 {
            return Some(offset + 2);
        }
        offset = offset.checked_add(length + 1)?;
        if offset >= buffer.len() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_query() {
        let packet = build_query(0x1234, "claude.ai").expect("query");
        assert_eq!(&packet[0..2], &[0x12, 0x34]);
        assert_eq!(&packet[12..13], &[6]);
        assert_eq!(&packet[19..20], &[2]);
        assert_eq!(packet.len(), 12 + 15);
    }

    #[test]
    fn parses_compressed_response() {
        let response = [
            0x12, 0x34, 0x81, 0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x06, b'c',
            b'l', b'a', b'u', b'd', b'e', 0x02, b'a', b'i', 0x00, 0x00, 0x01, 0x00, 0x01, 0xC0,
            0x0C, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3C, 0x00, 0x04, 0x01, 0x02, 0x03,
            0x04,
        ];
        let addresses = parse_answers(&response).expect("answers");
        assert_eq!(addresses, vec![Ipv4Addr::new(1, 2, 3, 4)]);
    }
}
