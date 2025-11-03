use crate::master_node::utils::MCPlayerUUID;
use crate::network::{
    EIOError, EIOReadExactError, MCPacketBuffer, ReadExt, ReadNumPrimitive, ReadUTF8,
    ReadUTF8Error, ReadUUID, ReadVarInt, ReadVarIntError, WriteMCPacket, WriteNumPrimitive,
    WriteUTF8, WriteUUID, WriteVarInt,
};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use core::cmp::{max, min};
use core::error::Error;
use core::fmt::{Debug, Display, Formatter};
use core::net::SocketAddr;
use defmt_or_log::*;
use embedded_io_async::Write;
use num_traits::abs;
use uuid::Uuid;

#[derive(Debug)]
pub struct MCClient<'a, T: embedded_io_async::Read + embedded_io_async::Write> {
    socket: &'a mut T,
    addr: Option<SocketAddr>,
    state: State,
    // player_name: String,
}

impl<T: embedded_io_async::Read + embedded_io_async::Write> Display for MCClient<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "MCClient({:?}, state: {:?})", self.addr, self.state)
    }
}

impl<'a, T: embedded_io_async::Read + embedded_io_async::Write> MCClient<'a, T>
where
    T::Error: 'static,
{
    pub fn new(socket: &'a mut T, addr: Option<SocketAddr>) -> Self {
        Self {
            socket,
            addr,
            state: State::Handshaking,
        }
    }

    async fn skip_unknown_packet(
        &mut self,
        packet_type: i32,
        packet_length: usize,
    ) -> Result<(), EIOReadExactError<T::Error>> {
        warn!(
            "{} received unknown packet (type: {}, length: {}), skipping.",
            self, packet_type, packet_length
        );
        let mut varintlen = 0;
        let mut pt = packet_type;
        while pt != 0 {
            pt >>= 7;
            varintlen += 1;
        }
        varintlen = max(varintlen, 1);
        self.socket.skip_bytes(packet_length - varintlen).await
    }

    async fn process_packet(&mut self) -> Result<(), MCClientError>
    where
        <T as embedded_io_async::ErrorType>::Error: 'static,
    {
        let packet_length: usize = self.socket.read_varint::<i32>().await? as usize;
        let packet_type = self.socket.read_varint::<i32>().await?;
        debug!(
            "{} recv pkt: type: {}, length: {}",
            self, packet_type, packet_length
        );
        match self.state {
            State::Handshaking => match packet_type {
                0 => {
                    let protocol_version = self.socket.read_varint::<i32>().await?;
                    let server_address = self.socket.read_utf8().await?;
                    let server_port = self.socket.read_be::<u16>().await?;
                    let intent = self.socket.read_varint::<i32>().await?;
                    debug!(
                        "{:?}, {:?}, {:?}, {:?}",
                        protocol_version, server_address, server_port, intent
                    );
                    let new_state = match intent {
                        1 => State::Status,
                        2 => State::Login,
                        3 => State::Login, // TODO: transfer login?
                        _ => {
                            return Err(MCClientError::ProtocolError(format!(
                                "Handshaking: invalid intent id: {intent}."
                            )));
                        }
                    };
                    debug!("{} Switching connection state to {:?}", self, new_state);
                    self.state = new_state;
                }
                _ => {
                    return Err(MCClientError::ProtocolError(format!(
                        "Received invalid packet of type {packet_type} before handshake."
                    )));
                }
            },
            State::Status => match packet_type {
                0 => {
                    // minecraft:status_request
                    let mut pkt = MCPacketBuffer::new(0).await;
                    pkt.write_utf8(r#"{"version":{"name":"1.21.8","protocol":772}, "description":{"text":"Hello, world!"}}"#).await?;
                    self.socket.write_mc_packet(pkt).await?;
                }
                1 => {
                    // minecraft:ping_request
                    let timestamp = self.socket.read_be::<i64>().await?;
                    let mut pkt = MCPacketBuffer::new(1).await;
                    pkt.write_be(timestamp).await?;
                    self.socket.write_mc_packet(pkt).await?;
                }
                _ => {
                    self.skip_unknown_packet(packet_type, packet_length).await?;
                }
            },
            State::Login => match packet_type {
                0 => {
                    // login start (minecraft:hello)
                    let player_name = self.socket.read_utf8().await?;
                    let given_player_uuid = self.socket.read_uuid().await?;
                    let player_uuid = Uuid::new_mc_offline_player(&player_name);

                    let mut pkt = MCPacketBuffer::new(2).await; // minecraft:login_finished
                    pkt.write_uuid(player_uuid).await?;
                    pkt.write_utf8(&player_name).await?;
                    pkt.write_varint::<u32>(0).await?; // empty properties array
                    self.socket.write_mc_packet(pkt).await?;
                }
                3 => {
                    // minecraft:login_acknowledged
                    debug!(
                        "{} Switching connection state to {:?}",
                        self,
                        State::Configuration
                    );
                    self.state = State::Configuration;

                    let mut pkt = MCPacketBuffer::new(14).await; // minecraft:select_known_packs
                    pkt.write_varint(1u32).await?;
                    pkt.write_utf8("minecraft").await?;
                    pkt.write_utf8("core").await?;
                    pkt.write_utf8("1.21.8").await?;
                    self.socket.write_mc_packet(pkt).await?;

                    // let mut pkt = MCPacketBuffer::new(7).await; // minecraft:registry_data
                    //
                    // self.socket.write_mc_packet(pkt).await?;

                    self.socket.write_all(&REGISTRY_PACKETS_DATA).await?;

                    self.socket
                        .write_mc_packet(MCPacketBuffer::new(3).await)
                        .await?; // minecraft:finish_configuration

                    info!("sguoid")
                }
                _ => {
                    self.skip_unknown_packet(packet_type, packet_length).await?;
                }
            },
            State::Configuration => match packet_type {
                3 => {
                    // minecraft:finish_configuration
                    debug!("{} Switching connection state to {:?}", self, State::Play);
                    self.state = State::Play;

                    let mut pkt = MCPacketBuffer::new(43).await; // minecraft:login
                    pkt.write_be::<u32>(0).await?; // entity id
                    pkt.write_be(false as u8).await?; // is hardcore
                    pkt.write_varint(1u32).await?; // dimension names
                    pkt.write_utf8("minecraft:overworld").await?;
                    pkt.write_varint(3u32).await?; // max players
                    pkt.write_varint(32u32).await?; // view distance
                    pkt.write_varint(32u32).await?; // sim distance
                    pkt.write_be(false as u8).await?; // reduced debug info
                    pkt.write_be(true as u8).await?; // enable respawn screen
                    pkt.write_be(false as u8).await?; // do limited crafting (unused)
                    pkt.write_varint(0u32).await?; // dimension id
                    pkt.write_utf8("minecraft:overworld").await?; // dimension name
                    pkt.write_be(0u64).await?; // world seed hash
                    pkt.write_be(1u8).await?; // game mode (0: Survival, 1: Creative, 2: Adventure, 3: Spectator)
                    pkt.write_be(-1i8).await?; // prev game mode
                    pkt.write_be(false as u8).await?; // is debug mode world
                    pkt.write_be(false as u8).await?; // is flat world
                    pkt.write_be(false as u8).await?; // has death location
                    pkt.write_varint(0u32).await?; // portal cooldown
                    pkt.write_varint(68u32).await?; // sea level
                    pkt.write_be(false as u8).await?; // enforce secure chat
                    self.socket.write_mc_packet(pkt).await?;

                    let mut pkt = MCPacketBuffer::new(34).await; // minecraft:game_event
                    pkt.write_be::<u8>(13).await?; // Start waiting for level chunks
                    pkt.write_be::<f32>(0.0).await?;
                    self.socket.write_mc_packet(pkt).await?;

                    let mut pkt = MCPacketBuffer::new(65).await; // minecraft:player_position
                    pkt.write_varint::<u32>(0).await?;
                    pkt.write_be::<f64>(0.0).await?;
                    pkt.write_be::<f64>(400.0).await?;
                    pkt.write_be::<f64>(0.0).await?;
                    pkt.write_be::<f64>(0.0).await?;
                    pkt.write_be::<f64>(0.0).await?;
                    pkt.write_be::<f64>(0.0).await?;
                    pkt.write_be::<f32>(0.0).await?;
                    pkt.write_be::<f32>(0.0).await?;
                    pkt.write_be::<u32>(0).await?;
                    self.socket.write_mc_packet(pkt).await?;

                    let mut pkt = MCPacketBuffer::new(87).await; // set_chunk_cache_center
                    pkt.write_varint(0u32).await?;
                    pkt.write_varint(0u32).await?;
                    self.socket.write_mc_packet(pkt).await?;

                    for x in -5..=5 {
                        for z in -5..=5 {
                            let mut pkt = MCPacketBuffer::new(39).await; // level_chunk_with_light
                            pkt.write_be::<i32>(x).await?;
                            pkt.write_be::<i32>(z).await?;

                            // chunk
                            pkt.write_varint(0u32).await?; // heightmaps
                            pkt.write_varint::<u32>((2+1+1+1+1) * 24).await?; // bytes
                            for y in -4..20 {
                                // blocks
                                if y == 4 && abs(x) <= 4 && abs(z) <= 4 {
                                    pkt.write_be(4096u16).await?;
                                    pkt.write_be(0u8).await?;
                                    pkt.write_varint(10u32).await?; // dirt
                                } else {
                                    pkt.write_be(0u16).await?;
                                    pkt.write_be(0u8).await?;
                                    pkt.write_varint(0u32).await?;
                                }
                                // biomes
                                pkt.write_be(0u8).await?;
                                pkt.write_varint(0u32).await?;
                            }
                            pkt.write_varint(0u32).await?; // block entities

                            // light
                            for _ in 0..2 {
                                pkt.write_varint(7u32).await?;
                                for _ in 0..7 {
                                    // pkt.write_be(0xFFFF_FFFF_FFFF_FFFFu64).await?;
                                    pkt.write_be(0u64).await?;
                                }
                            }
                            for _ in 0..2 {
                                pkt.write_varint(7u32).await?;
                                for _ in 0..7 {
                                    pkt.write_be(0xFFFF_FFFF_FFFF_FFFFu64).await?;
                                }
                            }
                            pkt.write_varint(0u32).await?;
                            pkt.write_varint(0u32).await?;

                            self.socket.write_mc_packet(pkt).await?;
                        }
                    }
                }
                _ => {
                    self.skip_unknown_packet(packet_type, packet_length).await?;
                }
            },
            State::Play => match packet_type {
                _ => {
                    self.skip_unknown_packet(packet_type, packet_length).await?;
                }
            },
            _ => {}
        }
        Ok(())
    }

    pub async fn _main_task(&mut self) {
        loop {
            if let Err(err) = self.process_packet().await {
                warn!("{} error: {:?}", self, err);
                break;
                // TODO: disconnection packet
            }
        }

        // let mut buf = [0u8; 1024];
        // loop {
        //     let n = match self.socket.read(&mut buf).await {
        //         Ok(0) => {
        //             warn!("Read EOF");
        //             return;
        //         }
        //         Ok(n) => n,
        //         Err(err) => {
        //             warn!("Read failed: {:?}", err);
        //             return;
        //         }
        //     };
        //     debug!(
        //         "Read: {}",
        //         defmt_or_log::wrappers::Hex(
        //             buf[..n]
        //                 .iter()
        //                 .map(|b| format!("{:02X}", b))
        //                 .collect::<Vec<_>>()
        //                 .join(" ")
        //         )
        //     );
        // }
    }
}

#[derive(Debug)]
enum State {
    Handshaking,
    Status,
    Login,
    Configuration,
    Play,
}

#[derive(Debug, derive_more::Display)]
#[display("{self:?}")]
pub enum MCClientError {
    /// Logical protocol error (e.g. wrong packet sequence)
    ProtocolError(String),
    /// Packet data format error (e.g. varint too big)
    DataError(Box<dyn Error>),
    NetworkError(Box<dyn Error>),
}

impl<IOE: embedded_io_async::Error + 'static> From<EIOError<IOE>> for MCClientError {
    fn from(value: EIOError<IOE>) -> Self {
        Self::NetworkError(Box::new(value))
    }
}

impl Error for MCClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ProtocolError(_) => None,
            Self::DataError(err) => Some(err.as_ref()),
            Self::NetworkError(err) => Some(err.as_ref()),
        }
    }
}

impl<E: embedded_io_async::Error + 'static> From<E> for MCClientError {
    fn from(value: E) -> Self {
        Self::NetworkError(EIOError(value).into())
    }
}

impl From<ReadVarIntError> for MCClientError {
    fn from(value: ReadVarIntError) -> Self {
        match value {
            ReadVarIntError::TooBig { .. } => MCClientError::DataError(Box::new(value)),
            ReadVarIntError::IOError(err) => MCClientError::NetworkError(err),
        }
    }
}

impl<E: Debug + 'static> From<EIOReadExactError<E>> for MCClientError {
    fn from(value: EIOReadExactError<E>) -> Self {
        Self::NetworkError(value.into())
    }
}

impl From<ReadUTF8Error> for MCClientError {
    fn from(value: ReadUTF8Error) -> Self {
        match value {
            ReadUTF8Error::ProtocolError(_) | ReadUTF8Error::UnicodeError(_) => {
                Self::DataError(value.into())
            }
            ReadUTF8Error::IOError(err) => Self::NetworkError(err.into()),
        }
    }
}

const REGISTRY_PACKETS_DATA: [u8; 942] = [
    0x42, 0x07, 0x0e, 0x64, 0x69, 0x6d, 0x65, 0x6e, 0x73, 0x69, 0x6f, 0x6e, 0x5f, 0x74, 0x79, 0x70,
    0x65, 0x04, 0x09, 0x6f, 0x76, 0x65, 0x72, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x00, 0x0f, 0x6f, 0x76,
    0x65, 0x72, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x5f, 0x63, 0x61, 0x76, 0x65, 0x73, 0x00, 0x07, 0x74,
    0x68, 0x65, 0x5f, 0x65, 0x6e, 0x64, 0x00, 0x0a, 0x74, 0x68, 0x65, 0x5f, 0x6e, 0x65, 0x74, 0x68,
    0x65, 0x72, 0x00, 0x13, 0x07, 0x0b, 0x63, 0x61, 0x74, 0x5f, 0x76, 0x61, 0x72, 0x69, 0x61, 0x6e,
    0x74, 0x01, 0x03, 0x72, 0x65, 0x64, 0x00, 0x1d, 0x07, 0x0f, 0x63, 0x68, 0x69, 0x63, 0x6b, 0x65,
    0x6e, 0x5f, 0x76, 0x61, 0x72, 0x69, 0x61, 0x6e, 0x74, 0x01, 0x09, 0x74, 0x65, 0x6d, 0x70, 0x65,
    0x72, 0x61, 0x74, 0x65, 0x00, 0x19, 0x07, 0x0b, 0x63, 0x6f, 0x77, 0x5f, 0x76, 0x61, 0x72, 0x69,
    0x61, 0x6e, 0x74, 0x01, 0x09, 0x74, 0x65, 0x6d, 0x70, 0x65, 0x72, 0x61, 0x74, 0x65, 0x00, 0x1a,
    0x07, 0x0c, 0x66, 0x72, 0x6f, 0x67, 0x5f, 0x76, 0x61, 0x72, 0x69, 0x61, 0x6e, 0x74, 0x01, 0x09,
    0x74, 0x65, 0x6d, 0x70, 0x65, 0x72, 0x61, 0x74, 0x65, 0x00, 0x18, 0x07, 0x10, 0x70, 0x61, 0x69,
    0x6e, 0x74, 0x69, 0x6e, 0x67, 0x5f, 0x76, 0x61, 0x72, 0x69, 0x61, 0x6e, 0x74, 0x01, 0x03, 0x6f,
    0x72, 0x62, 0x00, 0x19, 0x07, 0x0b, 0x70, 0x69, 0x67, 0x5f, 0x76, 0x61, 0x72, 0x69, 0x61, 0x6e,
    0x74, 0x01, 0x09, 0x74, 0x65, 0x6d, 0x70, 0x65, 0x72, 0x61, 0x74, 0x65, 0x00, 0x1a, 0x07, 0x12,
    0x77, 0x6f, 0x6c, 0x66, 0x5f, 0x73, 0x6f, 0x75, 0x6e, 0x64, 0x5f, 0x76, 0x61, 0x72, 0x69, 0x61,
    0x6e, 0x74, 0x01, 0x03, 0x62, 0x69, 0x67, 0x00, 0x15, 0x07, 0x0c, 0x77, 0x6f, 0x6c, 0x66, 0x5f,
    0x76, 0x61, 0x72, 0x69, 0x61, 0x6e, 0x74, 0x01, 0x04, 0x70, 0x61, 0x6c, 0x65, 0x00, 0xd7, 0x04,
    0x07, 0x0b, 0x64, 0x61, 0x6d, 0x61, 0x67, 0x65, 0x5f, 0x74, 0x79, 0x70, 0x65, 0x31, 0x05, 0x61,
    0x72, 0x72, 0x6f, 0x77, 0x00, 0x11, 0x62, 0x61, 0x64, 0x5f, 0x72, 0x65, 0x73, 0x70, 0x61, 0x77,
    0x6e, 0x5f, 0x70, 0x6f, 0x69, 0x6e, 0x74, 0x00, 0x06, 0x63, 0x61, 0x63, 0x74, 0x75, 0x73, 0x00,
    0x08, 0x63, 0x61, 0x6d, 0x70, 0x66, 0x69, 0x72, 0x65, 0x00, 0x08, 0x63, 0x72, 0x61, 0x6d, 0x6d,
    0x69, 0x6e, 0x67, 0x00, 0x0d, 0x64, 0x72, 0x61, 0x67, 0x6f, 0x6e, 0x5f, 0x62, 0x72, 0x65, 0x61,
    0x74, 0x68, 0x00, 0x05, 0x64, 0x72, 0x6f, 0x77, 0x6e, 0x00, 0x07, 0x64, 0x72, 0x79, 0x5f, 0x6f,
    0x75, 0x74, 0x00, 0x0b, 0x65, 0x6e, 0x64, 0x65, 0x72, 0x5f, 0x70, 0x65, 0x61, 0x72, 0x6c, 0x00,
    0x09, 0x65, 0x78, 0x70, 0x6c, 0x6f, 0x73, 0x69, 0x6f, 0x6e, 0x00, 0x04, 0x66, 0x61, 0x6c, 0x6c,
    0x00, 0x0d, 0x66, 0x61, 0x6c, 0x6c, 0x69, 0x6e, 0x67, 0x5f, 0x61, 0x6e, 0x76, 0x69, 0x6c, 0x00,
    0x0d, 0x66, 0x61, 0x6c, 0x6c, 0x69, 0x6e, 0x67, 0x5f, 0x62, 0x6c, 0x6f, 0x63, 0x6b, 0x00, 0x12,
    0x66, 0x61, 0x6c, 0x6c, 0x69, 0x6e, 0x67, 0x5f, 0x73, 0x74, 0x61, 0x6c, 0x61, 0x63, 0x74, 0x69,
    0x74, 0x65, 0x00, 0x08, 0x66, 0x69, 0x72, 0x65, 0x62, 0x61, 0x6c, 0x6c, 0x00, 0x09, 0x66, 0x69,
    0x72, 0x65, 0x77, 0x6f, 0x72, 0x6b, 0x73, 0x00, 0x0d, 0x66, 0x6c, 0x79, 0x5f, 0x69, 0x6e, 0x74,
    0x6f, 0x5f, 0x77, 0x61, 0x6c, 0x6c, 0x00, 0x06, 0x66, 0x72, 0x65, 0x65, 0x7a, 0x65, 0x00, 0x07,
    0x67, 0x65, 0x6e, 0x65, 0x72, 0x69, 0x63, 0x00, 0x0c, 0x67, 0x65, 0x6e, 0x65, 0x72, 0x69, 0x63,
    0x5f, 0x6b, 0x69, 0x6c, 0x6c, 0x00, 0x09, 0x68, 0x6f, 0x74, 0x5f, 0x66, 0x6c, 0x6f, 0x6f, 0x72,
    0x00, 0x0e, 0x69, 0x6e, 0x64, 0x69, 0x72, 0x65, 0x63, 0x74, 0x5f, 0x6d, 0x61, 0x67, 0x69, 0x63,
    0x00, 0x07, 0x69, 0x6e, 0x5f, 0x66, 0x69, 0x72, 0x65, 0x00, 0x07, 0x69, 0x6e, 0x5f, 0x77, 0x61,
    0x6c, 0x6c, 0x00, 0x04, 0x6c, 0x61, 0x76, 0x61, 0x00, 0x0e, 0x6c, 0x69, 0x67, 0x68, 0x74, 0x6e,
    0x69, 0x6e, 0x67, 0x5f, 0x62, 0x6f, 0x6c, 0x74, 0x00, 0x0a, 0x6d, 0x61, 0x63, 0x65, 0x5f, 0x73,
    0x6d, 0x61, 0x73, 0x68, 0x00, 0x05, 0x6d, 0x61, 0x67, 0x69, 0x63, 0x00, 0x0a, 0x6d, 0x6f, 0x62,
    0x5f, 0x61, 0x74, 0x74, 0x61, 0x63, 0x6b, 0x00, 0x13, 0x6d, 0x6f, 0x62, 0x5f, 0x61, 0x74, 0x74,
    0x61, 0x63, 0x6b, 0x5f, 0x6e, 0x6f, 0x5f, 0x61, 0x67, 0x67, 0x72, 0x6f, 0x00, 0x0e, 0x6d, 0x6f,
    0x62, 0x5f, 0x70, 0x72, 0x6f, 0x6a, 0x65, 0x63, 0x74, 0x69, 0x6c, 0x65, 0x00, 0x07, 0x6f, 0x6e,
    0x5f, 0x66, 0x69, 0x72, 0x65, 0x00, 0x0e, 0x6f, 0x75, 0x74, 0x73, 0x69, 0x64, 0x65, 0x5f, 0x62,
    0x6f, 0x72, 0x64, 0x65, 0x72, 0x00, 0x0c, 0x6f, 0x75, 0x74, 0x5f, 0x6f, 0x66, 0x5f, 0x77, 0x6f,
    0x72, 0x6c, 0x64, 0x00, 0x0d, 0x70, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x5f, 0x61, 0x74, 0x74, 0x61,
    0x63, 0x6b, 0x00, 0x10, 0x70, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x5f, 0x65, 0x78, 0x70, 0x6c, 0x6f,
    0x73, 0x69, 0x6f, 0x6e, 0x00, 0x0a, 0x73, 0x6f, 0x6e, 0x69, 0x63, 0x5f, 0x62, 0x6f, 0x6f, 0x6d,
    0x00, 0x04, 0x73, 0x70, 0x69, 0x74, 0x00, 0x0a, 0x73, 0x74, 0x61, 0x6c, 0x61, 0x67, 0x6d, 0x69,
    0x74, 0x65, 0x00, 0x06, 0x73, 0x74, 0x61, 0x72, 0x76, 0x65, 0x00, 0x05, 0x73, 0x74, 0x69, 0x6e,
    0x67, 0x00, 0x10, 0x73, 0x77, 0x65, 0x65, 0x74, 0x5f, 0x62, 0x65, 0x72, 0x72, 0x79, 0x5f, 0x62,
    0x75, 0x73, 0x68, 0x00, 0x06, 0x74, 0x68, 0x6f, 0x72, 0x6e, 0x73, 0x00, 0x06, 0x74, 0x68, 0x72,
    0x6f, 0x77, 0x6e, 0x00, 0x07, 0x74, 0x72, 0x69, 0x64, 0x65, 0x6e, 0x74, 0x00, 0x15, 0x75, 0x6e,
    0x61, 0x74, 0x74, 0x72, 0x69, 0x62, 0x75, 0x74, 0x65, 0x64, 0x5f, 0x66, 0x69, 0x72, 0x65, 0x62,
    0x61, 0x6c, 0x6c, 0x00, 0x0b, 0x77, 0x69, 0x6e, 0x64, 0x5f, 0x63, 0x68, 0x61, 0x72, 0x67, 0x65,
    0x00, 0x06, 0x77, 0x69, 0x74, 0x68, 0x65, 0x72, 0x00, 0x0c, 0x77, 0x69, 0x74, 0x68, 0x65, 0x72,
    0x5f, 0x73, 0x6b, 0x75, 0x6c, 0x6c, 0x00, 0x46, 0x07, 0x0e, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x67,
    0x65, 0x6e, 0x2f, 0x62, 0x69, 0x6f, 0x6d, 0x65, 0x05, 0x06, 0x70, 0x6c, 0x61, 0x69, 0x6e, 0x73,
    0x00, 0x0e, 0x6d, 0x61, 0x6e, 0x67, 0x72, 0x6f, 0x76, 0x65, 0x5f, 0x73, 0x77, 0x61, 0x6d, 0x70,
    0x00, 0x06, 0x64, 0x65, 0x73, 0x65, 0x72, 0x74, 0x00, 0x0c, 0x73, 0x6e, 0x6f, 0x77, 0x79, 0x5f,
    0x70, 0x6c, 0x61, 0x69, 0x6e, 0x73, 0x00, 0x05, 0x62, 0x65, 0x61, 0x63, 0x68, 0x00,
];
