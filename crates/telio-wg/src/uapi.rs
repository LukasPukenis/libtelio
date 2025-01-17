use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use telio_crypto::{PublicKey, SecretKey};
use telio_utils::telio_log_warn;
use wireguard_uapi::{get, xplatform::set};

use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    io::{BufRead, BufReader, Read},
    net::SocketAddr,
    panic,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant, SystemTime, SystemTimeError, UNIX_EPOCH},
};

/// telio implementation of wireguard::Peer
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Peer {
    /// Public key, the peer's primary identifier
    pub public_key: PublicKey,
    /// Peer's endpoint with `IP address` and `UDP port` number
    pub endpoint: Option<SocketAddr>,
    /// Keep alive interval, `seconds` or `None`
    pub persistent_keepalive_interval: Option<u32>,
    /// Vector of allowed IPs
    pub allowed_ips: Vec<IpNetwork>,
    /// Number of bytes received or `None`(unused on Set)
    pub rx_bytes: Option<u64>,
    /// Number of bytes transmitted or `None`(unused on Set)
    pub tx_bytes: Option<u64>,
    /// Time since last handshakeor `None`, differs from WireGuard field meaning
    pub time_since_last_handshake: Option<Duration>,
}

impl From<get::Peer> for Peer {
    /// Convert from WireGuard get::Peer to telio Peer
    fn from(item: get::Peer) -> Self {
        Self {
            public_key: PublicKey(item.public_key),
            endpoint: item.endpoint,
            persistent_keepalive_interval: Some(item.persistent_keepalive_interval.into()),
            allowed_ips: item
                .allowed_ips
                .into_iter()
                .map(|ip| IpNetwork::new(ip.ipaddr, ip.cidr_mask))
                .collect::<Result<Vec<IpNetwork>, _>>()
                .unwrap_or_default(),
            rx_bytes: Some(item.rx_bytes),
            tx_bytes: Some(item.tx_bytes),
            time_since_last_handshake: Peer::calculate_time_since_last_handshake(Some(
                item.last_handshake_time,
            )),
        }
    }
}

impl From<set::Peer> for Peer {
    /// Convert from WireGuard set::Peer to telio Peer
    fn from(item: set::Peer) -> Self {
        Self {
            public_key: PublicKey(item.public_key),
            endpoint: item.endpoint,
            persistent_keepalive_interval: item.persistent_keepalive_interval.map(u32::from),
            allowed_ips: item
                .allowed_ips
                .into_iter()
                .map(|ip| IpNetwork::new(ip.ipaddr, ip.cidr_mask))
                .collect::<Result<Vec<IpNetwork>, _>>()
                .unwrap_or_default(),
            ..Default::default()
        }
    }
}

impl From<&Peer> for set::Peer {
    /// Convert from telio Peer to WireGuard set::Peer
    fn from(item: &Peer) -> Self {
        Self {
            public_key: item.public_key.0,
            endpoint: item.endpoint,
            persistent_keepalive_interval: item.persistent_keepalive_interval.map(|x| x as u16),
            allowed_ips: item
                .allowed_ips
                .iter()
                .map(|ip| set::AllowedIp {
                    ipaddr: ip.network(),
                    cidr_mask: ip.prefix(),
                })
                .collect(),
            ..Default::default()
        }
    }
}

/// telio-wg representation of WireGuard Interface
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Interface {
    /// Private key or `None`
    pub private_key: Option<SecretKey>,
    /// Listen port or `None`
    pub listen_port: Option<u16>,
    /// firewall mark
    pub fwmark: u32,
    /// Dictionary of Peer-s
    pub peers: BTreeMap<PublicKey, Peer>,
}

impl From<get::Device> for Interface {
    /// Convert from wireguard get::Device to telio Interface
    fn from(item: get::Device) -> Self {
        Self {
            private_key: item.private_key.map(SecretKey),
            listen_port: Some(item.listen_port),
            fwmark: item.fwmark,
            peers: item
                .peers
                .into_iter()
                .map(|p| (PublicKey(p.public_key), Peer::from(p)))
                .collect(),
        }
    }
}

impl From<set::Device> for Interface {
    /// Convert from wireguard set::Device to telio Interface
    fn from(item: set::Device) -> Self {
        Self {
            private_key: item.private_key.map(SecretKey),
            listen_port: item.listen_port,
            fwmark: item.fwmark.map_or(0, |x| x),
            peers: item
                .peers
                .into_iter()
                .map(|p| (PublicKey(p.public_key), Peer::from(p)))
                .collect(),
        }
    }
}

impl From<Interface> for set::Device {
    /// Convert from telio Interface to wireguard set::Device
    fn from(item: Interface) -> Self {
        Self {
            private_key: item.private_key.map(|key| key.0),
            listen_port: item.listen_port,
            fwmark: match item.fwmark {
                0 => None,
                x => Some(x),
            },
            peers: item.peers.values().map(Into::<set::Peer>::into).collect(),
            ..Default::default()
        }
    }
}

/// Types of commands
#[derive(Debug, PartialEq)]
pub enum Cmd {
    /// Get command has no underlying structure
    Get,
    /// Set command is wrapping WireGuard set::Device
    Set(set::Device),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Response {
    pub errno: i32,
    pub interface: Option<Interface>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerState {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Event {
    pub state: PeerState,
    pub peer: Peer,
}

#[derive(Clone, Debug)]
pub struct AnalyticsEvent {
    pub public_key: PublicKey,
    pub endpoint: SocketAddr,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub peer_state: PeerState,
    pub timestamp: Instant,
}

impl Peer {
    /// Represents 2022-03-04 17:00:05
    #[cfg(test)]
    const MOCK_UNIX_TIME: Duration = Duration::from_secs(1646405984);

    pub fn connected(&self) -> bool {
        // https://web.archive.org/web/20200603205723/https://www.wireguard.com/papers/wireguard.pdf
        // 6.1
        const REJECT_AFTER_TIME: Duration = Duration::from_secs(180);
        // Whenever a handshake initiation message is sent as the result of an
        // expiring timer, an additional amount of jitter is added to the
        // expiration, in order to prevent two peers from repeatedly initiating
        // handshakes at the same time.
        //
        // ernestask: Canonical implementations use a third of a second, but at
        //            least wireguard-go rounds up.
        const REKEY_TIMEOUT_JITTER: Duration = Duration::from_millis(334);
        // 6.2
        // However, for the case in which a peer has received data but does not
        // have any data to send back immediately, and the Reject-After-Time
        // second deadline is approaching in sooner than Keepalive-Timeout
        // seconds, then the initiation triggered by an aged secure session
        // occurs during the receive path.
        //
        // 6.4
        // After sending a handshake initiation message, because of a
        // first-packet condition, or because of the limit conditions of
        // section 6.2, if a handshake response message (section 5.4.3) is not
        // subsequently received after Rekey-Timeout seconds, a new handshake
        // initiation message is constructed (with new random ephemeral keys)
        // and sent. This reinitiation is attempted for Rekey-Attempt-Time
        // seconds before giving up, though this counter is reset when a peer
        // explicitly attempts to send a new transport data message.
        //
        // ernestask: With the above in mind, the hard deadline for rekeying is
        //            Reject-After-Time - Rekey-Timeout + Rekey-Attempt-Time,
        //            although wireguard-go seems to implement it in a way that
        //            ends up being Reject-After-Time + Rekey-Attempt-Time.
        //
        //            However, since this mostly pertains to judging whether
        //            the peer is connect_ed_ vs connect_ing_, simply using
        //            Reject-After-Time + jitter should be fine.

        self.time_since_last_handshake
            .map_or(false, |d| d < REJECT_AFTER_TIME + REKEY_TIMEOUT_JITTER)
    }

    #[cfg(not(test))]
    fn get_unix_time() -> Result<Duration, SystemTimeError> {
        SystemTime::now().duration_since(UNIX_EPOCH)
    }

    #[cfg(test)]
    fn get_unix_time() -> Result<Duration, SystemTimeError> {
        Ok(Self::MOCK_UNIX_TIME)
    }

    /// Convert uapi last_handshake_time into Duration since handshake
    pub fn calculate_time_since_last_handshake(lht: Option<Duration>) -> Option<Duration> {
        // 0 means no handshake
        let lht = lht.and_then(|handshake_time| {
            if handshake_time == Duration::from_secs(0) {
                None
            } else {
                Some(handshake_time)
            }
        });

        lht.and_then(|handshake_time| match Self::get_unix_time() {
            Ok(now) => now.checked_sub(handshake_time),
            Err(e) => {
                telio_log_warn!("Failed to parse unix_time for peer: {}", e);
                None
            }
        })
    }
}

impl Display for Cmd {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Cmd::Get => writeln!(f, "get=1"),
            Cmd::Set(interface) => {
                writeln!(f, "set=1")?;
                writeln!(f, "{}", interface)
            }
        }
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.errno {
            e if e != 0 => {
                writeln!(f, "error={}", e)?;
            }
            _ => (),
        }
        Ok(())
    }
}

impl Default for PeerState {
    fn default() -> Self {
        PeerState::Disconnected
    }
}

pub(super) fn response_from_str(string: &str) -> Response {
    response_from_read(string.as_bytes())
}

#[allow(unwrap_check)]
pub(super) fn response_from_read<R: Read>(reader: R) -> Response {
    let mut reader = BufReader::new(reader);
    let mut interface = Interface::default();
    let mut inited = false;
    let mut errno = 0;
    let mut cmd = String::new();

    while reader.read_line(&mut cmd).is_ok() {
        cmd.pop(); // remove newline if any
        if cmd.is_empty() {
            return Response {
                errno,
                interface: if inited { Some(interface) } else { None },
            }; // Done
        }
        {
            let parsed: Vec<&str> = cmd.splitn(2, '=').collect();
            assert_eq!(parsed.len(), 2);
            let (key, val) = (parsed[0], parsed[1]);

            match key {
                "private_key" => {
                    inited = true;
                    interface.private_key = Some(val.parse().unwrap());
                }
                "listen_port" => {
                    inited = true;
                    let port = val.parse().unwrap();
                    if port > 0 {
                        interface.listen_port = Some(port);
                    }
                }
                "fwmark" => {
                    inited = true;
                    interface.fwmark = val.parse().unwrap();
                }
                "public_key" => {
                    inited = true;
                    let mut public = val.parse().unwrap();
                    loop {
                        let (peer, next, err) = parse_peer(public, &mut reader);
                        let _ = interface.peers.insert(peer.public_key, peer);
                        if let Some(err) = err {
                            errno = err;
                            break;
                        }
                        if let Some(next) = next {
                            public = next;
                        } else {
                            break;
                        }
                    }
                }
                "errno" => errno = val.parse().unwrap(),
                _ => (),
            }
        }
        cmd.clear();
    }

    Response {
        errno,
        interface: if inited { Some(interface) } else { None },
    }
}

#[allow(unwrap_check)]
fn parse_peer<R: Read>(
    public_key: PublicKey,
    reader: &mut BufReader<R>,
) -> (Peer, Option<PublicKey>, Option<i32>) {
    let mut cmd = String::new();

    let mut peer = Peer {
        public_key,
        ..Peer::default()
    };

    let mut last_handshake_time = None;
    let mut resp = loop {
        if reader.read_line(&mut cmd).is_err() {
            break (peer, None, None);
        }

        cmd.pop(); // remove newline if any
        if cmd.is_empty() {
            break (peer, None, None);
        }

        let parsed: Vec<&str> = cmd.splitn(2, '=').collect();
        assert_eq!(parsed.len(), 2);
        let (key, val) = (parsed[0], parsed[1]);

        match key {
            "endpoint" => peer.endpoint = Some(val.parse().unwrap()),
            "persistent_keepalive_interval" => {
                peer.persistent_keepalive_interval = Some(val.parse().unwrap())
            }
            "allowed_ip" => peer.allowed_ips.push(val.parse().unwrap()),
            "rx_bytes" => peer.rx_bytes = Some(val.parse().unwrap()),
            "tx_bytes" => peer.tx_bytes = Some(val.parse().unwrap()),
            "last_handshake_time_nsec" => {
                let nsec = Duration::from_nanos(val.parse().unwrap());
                if let Some(ref mut timestamp) = last_handshake_time {
                    *timestamp += nsec;
                } else {
                    last_handshake_time = Some(nsec);
                }
            }
            "last_handshake_time_sec" => {
                let sec = Duration::from_secs(val.parse().unwrap());
                if let Some(ref mut timestamp) = last_handshake_time {
                    *timestamp += sec;
                } else {
                    last_handshake_time = Some(sec);
                }
            }
            "public_key" => {
                break (
                    peer,
                    Some(val.parse().unwrap()), // Indicate next peer's public
                    None,
                );
            }
            "errno" => break (peer, None, Some(val.parse().unwrap())),
            _ => (),
        }
        cmd.clear();
    };

    resp.0.time_since_last_handshake =
        Peer::calculate_time_since_last_handshake(last_handshake_time);

    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    trait PeerHelp {
        fn peer_map(self) -> BTreeMap<PublicKey, Peer>;
    }

    impl PeerHelp for Vec<Peer> {
        fn peer_map(self) -> BTreeMap<PublicKey, Peer> {
            self.into_iter().map(|p| (p.public_key, p)).collect()
        }
    }

    #[test]
    fn bytes_data_overflow() {
        let base_string = "\
private_key=aa920dc47dc5ed30e586263e83f387bc110f421b52377935012e480c04b054a3
listen_port=12912
public_key=980dc0012a52f21927d952e866fcfabe9db61d406bc333249cc4d729d37ecb44
endpoint=[abcd:23::33%2]:51820
allowed_ip=192.168.4.4/32
last_handshake_time_nsec=1234
last_handshake_time_sec=1
public_key=f1bc9c87d65731aecde6197c001f4219d839c2d384a47799af058bc3fbdab8f1
rx_bytes=RXBYTES
tx_bytes=TXBYTES
last_handshake_time_nsec=51204
last_handshake_time_sec=100
endpoint=182.122.22.19:3233
persistent_keepalive_interval=111
allowed_ip=192.168.4.10/32
allowed_ip=192.168.4.11/32
errno=0
";

        let received_bytes_overflow_string = base_string
            .replace("RXBYTES", "1000000000000")
            .replace("TXBYTES", "100"); // more than 4gb received in total
        let sent_bytes_overflow_string = base_string
            .replace("RXBYTES", "100")
            .replace("TXBYTES", "1000000000000"); // more than 4gb sent in total

        let result = panic::catch_unwind(|| {
            response_from_str(&received_bytes_overflow_string);
            response_from_str(&sent_bytes_overflow_string);
        });
        assert!(result.is_ok());
    }

    #[test]
    fn zero_listen_port_becomes_none() {
        let resp_str = "\
listen_port=0
errno=0
";
        let resp = Response {
            errno: 0,
            interface: Some(Interface::default()),
        };
        assert_eq!(response_from_str(&resp_str), resp);
    }
}
