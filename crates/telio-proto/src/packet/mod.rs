mod data;
mod generation;
mod natter;
mod nurse;
mod pinger;

use crate::{Codec, CodecError, CodecResult};

pub use self::{
    data::DataMsg, generation::Generation, natter::CallMeMaybeMsg,
    natter::CallMeMaybeMsgDeprecated, nurse::HeartbeatMessage, pinger::PingerMsg,
    pinger::PingerMsgDeprecated, pinger::Timestamp,
};

use std::convert::TryFrom;

/// Default buffer capacity allocated for a packet
pub const MAX_PACKET_SIZE: usize = 65536;

/// Session number for pinger and natter messages
pub type Session = u64;

/// Unique id, generate from peer's public keys
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct PeerId(pub u16);

/// Unique id, generate from peer's public keys
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WGPort(pub u16);

/// Downcast packet to a more concreate type
pub trait DowncastPacket {
    /// Downcast packet into inner type or return enum on failure.
    fn downcast(packet: Packet) -> Result<Self, Packet>
    where
        Self: Sized;
}

/// Trait bound for any packet.
pub trait AnyPacket: Codec + DowncastPacket + Into<Packet> + Send {}

impl<T> AnyPacket for T where T: Codec + DowncastPacket + Into<Packet> + Send {}

#[repr(u8)]
#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone, strum::EnumIter, strum::FromRepr)]
/// Byte encoding of telio [Packet] types.
pub enum PacketType {
    /// Plain WG packet.
    Data = 0x00,
    /// Plain WG packet with generation index.
    GenData = 0x01,
    /// Meshnet heartbeat packet.
    Heartbeat = 0x02,
    /// CallMeMaybe messages from/to natter
    CallMeMaybeDeprecated = 0x03,
    /// Pinger packets (oneway, ping_latency, simple pings ...)
    PingerDeprecated = 0x04,
    /// Encrypted package which is not Data type
    Encrypted = 0x05,
    /// CallMeMaybe messages from/to natter
    CallMeMaybe = 0x06,
    /// Pinger packets (oneway, ping_latency, simple pings ...)
    Pinger = 0x07,
    /// Reserved for future, in case we use all byte values for types.
    Reserved = 0xfe,

    /// Packet is of invalid type.
    Invalid = 0xff,
}

impl From<u8> for PacketType {
    fn from(val: u8) -> Self {
        PacketType::from_repr(val).unwrap_or(PacketType::Invalid)
    }
}

/// Complete telio packet representation.
#[derive(Debug, PartialEq, Clone)]
pub enum Packet {
    /// Packet used to transfer WG packets.
    Data(DataMsg),
    /// Meshnet heartbeat packet.
    Heartbeat(HeartbeatMessage),
    /// Deprecated for natter <--> natter communications
    CallMeMaybeDeprecated(CallMeMaybeMsgDeprecated),
    /// Deprecated pinging and checking the remote endpoints
    PingerDeprecated(PingerMsgDeprecated),
    /// For natter <--> natter communications
    CallMeMaybe(CallMeMaybeMsg),
    /// Pinging and checking the remote endpoints
    Pinger(PingerMsg),
}

impl Codec for Packet {
    const TYPES: &'static [PacketType] = &[
        PacketType::Data,
        PacketType::GenData,
        PacketType::Heartbeat,
        PacketType::CallMeMaybeDeprecated,
        PacketType::PingerDeprecated,
        PacketType::CallMeMaybe,
        PacketType::Pinger,
    ];

    fn decode(bytes: &[u8]) -> CodecResult<Self>
    where
        Self: Sized,
    {
        use PacketType::*;

        if bytes.is_empty() {
            return Err(CodecError::InvalidLength);
        }

        match PacketType::from(bytes[0]) {
            Data | GenData => Ok(Self::Data(DataMsg::decode(bytes)?)),
            Heartbeat => Ok(Self::Heartbeat(HeartbeatMessage::decode(bytes)?)),
            CallMeMaybe => Ok(Self::CallMeMaybe(CallMeMaybeMsg::decode(bytes)?)),
            Pinger => Ok(Self::Pinger(PingerMsg::decode(bytes)?)),
            CallMeMaybeDeprecated => Ok(Self::CallMeMaybeDeprecated(
                CallMeMaybeMsgDeprecated::decode(bytes)?,
            )),
            PingerDeprecated => Ok(Self::PingerDeprecated(PingerMsgDeprecated::decode(bytes)?)),
            // At this point a package already should be decrypted if is not Data
            Reserved | Invalid | Encrypted => Err(CodecError::DecodeFailed),
        }
    }

    fn encode(self) -> CodecResult<Vec<u8>> {
        match self {
            Self::Data(msg) => msg.encode(),
            Self::Heartbeat(msg) => msg.encode(),
            Self::CallMeMaybe(msg) => msg.encode(),
            Self::Pinger(msg) => msg.encode(),
            Self::CallMeMaybeDeprecated(msg) => msg.encode(),
            Self::PingerDeprecated(msg) => msg.encode(),
        }
    }

    fn packet_type(&self) -> PacketType {
        match self {
            Self::Data(msg) => msg.packet_type(),
            Self::Heartbeat(msg) => msg.packet_type(),
            Self::CallMeMaybe(msg) => msg.packet_type(),
            Self::Pinger(msg) => msg.packet_type(),
            Self::CallMeMaybeDeprecated(msg) => msg.packet_type(),
            Self::PingerDeprecated(msg) => msg.packet_type(),
        }
    }
}

impl DowncastPacket for Packet {
    fn downcast(packet: Packet) -> Result<Self, Packet>
    where
        Self: Sized,
    {
        Ok(packet)
    }
}

impl TryFrom<&[u8]> for PeerId {
    type Error = CodecError;

    fn try_from(other: &[u8]) -> std::result::Result<Self, Self::Error> {
        if other.len() != std::mem::size_of::<PeerId>() {
            return Err(CodecError::DecodeFailed);
        }

        // Note: all data should be converted to netowrk endian (BE)
        Ok(Self(
            unsafe { std::mem::transmute::<[u8; 2], u16>([other[0], other[1]]) }.to_be(),
        ))
    }
}

impl TryFrom<&[u8]> for WGPort {
    type Error = CodecError;

    fn try_from(other: &[u8]) -> std::result::Result<Self, Self::Error> {
        if other.len() != std::mem::size_of::<WGPort>() {
            return Err(CodecError::DecodeFailed);
        }

        // Note: all data should be converted to netowrk endian (BE)
        let other_le = [other[0], other[1]];
        Ok(Self(u16::from_be_bytes(other_le)))
    }
}

impl From<DataMsg> for Packet {
    fn from(other: DataMsg) -> Self {
        Self::Data(other)
    }
}

impl From<CallMeMaybeMsgDeprecated> for Packet {
    fn from(other: CallMeMaybeMsgDeprecated) -> Self {
        Self::CallMeMaybeDeprecated(other)
    }
}

impl From<PingerMsgDeprecated> for Packet {
    fn from(other: PingerMsgDeprecated) -> Self {
        Self::PingerDeprecated(other)
    }
}

impl From<HeartbeatMessage> for Packet {
    fn from(other: HeartbeatMessage) -> Self {
        Self::Heartbeat(other)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn all_packet_types_are_covered() {
        assert_eq!(
            Packet::TYPES,
            &PacketType::iter()
                .filter(|pt| {
                    (pt != &PacketType::Reserved)
                        && (pt != &PacketType::Encrypted)
                        && (pt != &PacketType::Invalid)
                })
                .collect::<Vec<_>>()
        )
    }

    #[test]
    fn decode_empty_packet() {
        assert_eq!(Packet::decode(&[]), Err(CodecError::InvalidLength));
    }

    #[test]
    fn decode_invalid_packet() {
        let bytes = &[PacketType::Invalid as u8, 1, 2, 3];
        assert_eq!(Packet::decode(bytes), Err(CodecError::DecodeFailed));
    }

    #[test]
    fn decode_data_packet() {
        let bytes = &[0, 1, 2, 3];
        let expected: Packet = DataMsg::new(&[1, 2, 3]).into();
        assert_eq!(Packet::decode(bytes), Ok(expected));
    }

    #[test]
    fn encode_data_packet() {
        let packet: Packet = DataMsg::new(&[3, 2, 1]).into();
        let expected = &[0, 3, 2, 1];
        assert_eq!(&packet.encode().unwrap(), expected);
    }
}
