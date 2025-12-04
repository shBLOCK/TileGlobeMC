use crate::mc_server::MCServer;
use crate::player::Player;
use crate::utils::MCPlayerUUID;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use core::cmp::max;
use core::error::Error;
use core::fmt::{Debug, Formatter};
use core::net::SocketAddr;
use defmt_or_log::*;
use embassy_futures::select::Either;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Ticker};
use num_traits::{ToPrimitive, abs};
use tileglobe::world::ChunkPos;
use tileglobe::world::world::World;
use tileglobe_utils::network::{
    EIOError, EIOReadExactError, MCPacketBuffer, ReadExt, ReadNumPrimitive, ReadUTF8,
    ReadUTF8Error, ReadUUID, ReadVarInt, ReadVarIntError, WriteMCPacket, WriteNumPrimitive,
    WriteUTF8, WriteUUID, WriteVarInt,
};
use uuid::Uuid;

#[derive(derive_more::Display)]
#[display("{self:?}")]
pub struct MCClient<
    'a,
    M: RawMutex,
    RX: embedded_io_async::Read,
    TX: embedded_io_async::Write,
    WORLD: World,
    SM: RawMutex,
> {
    server: &'a MCServer<'a, SM, WORLD>,
    rx: Mutex<M, RX>,
    tx: Mutex<M, TX>,
    addr: Option<SocketAddr>,
    player_data: Option<PlayerData>,
}

struct PlayerData {
    uuid: Uuid,
    name: String,
}

impl<
    M: RawMutex,
    RX: embedded_io_async::Read,
    TX: embedded_io_async::Write,
    WORLD: World,
    SM: RawMutex,
> Player for MCClient<'_, M, RX, TX, WORLD, SM>
where
    RX::Error: 'static,
    TX::Error: 'static,
{
    fn uuid(&self) -> Uuid {
        self.player_data.as_ref().unwrap().uuid
    }

    async fn tick(&self) {
        self.send_chunk(ChunkPos::new(0, 0)).await;
    }
}

impl<
    M: RawMutex,
    RX: embedded_io_async::Read,
    TX: embedded_io_async::Write,
    WORLD: World,
    SM: RawMutex,
> Debug for MCClient<'_, M, RX, TX, WORLD, SM>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "MCClient({:?})", self.addr)
    }
}

#[cfg(feature = "defmt")]
impl<
    M: RawMutex,
    RX: embedded_io_async::Read,
    TX: embedded_io_async::Write,
    WORLD: World,
    SM: RawMutex,
> defmt::Format for MCClient<'_, M, RX, TX, WORLD, SM>
{
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "MCClient({:?})", Debug2Format(&self.addr),)
    }
}

impl<
    'a,
    M: RawMutex,
    RX: embedded_io_async::Read,
    TX: embedded_io_async::Write,
    WORLD: World,
    SM: RawMutex,
> MCClient<'a, M, RX, TX, WORLD, SM>
where
    RX::Error: 'static,
    TX::Error: 'static,
{
    async fn skip_unknown_packet(
        &self,
        rx: &mut RX,
        packet_type: i32,
        packet_length: usize,
    ) -> Result<(), EIOReadExactError<RX::Error>> {
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
        rx.skip_bytes(packet_length - varintlen).await
    }

    async fn write_mc_packet(&self, pkt: MCPacketBuffer) -> Result<(), EIOError<TX::Error>> {
        self.tx.lock().await.write_mc_packet(pkt).await
    }

    async fn read_mc_packet_header(&self, rx: &mut RX) -> Result<(usize, i32), MCClientError> {
        let packet_length = rx.read_varint::<i32>().await? as usize;
        let packet_type = rx.read_varint::<i32>().await?;
        debug!(
            "{} recv pkt: type: {}, length: {}",
            self, packet_type, packet_length
        );
        Ok((packet_length, packet_type))
    }

    async fn send_chunk(&self, pos: ChunkPos) -> Result<(), MCClientError> {
        let mut pkt = MCPacketBuffer::new(39).await; // level_chunk_with_light
        pkt.write_be::<i32>(pos.x.to_i32().unwrap()).await?;
        pkt.write_be::<i32>(pos.y.to_i32().unwrap()).await?;

        self.server.world.write_net_chunk(pos, &mut pkt).await?;

        self.write_mc_packet(pkt).await?;
        Ok(())
    }
}

impl<
    'a,
    M: RawMutex + 'static,
    RX: embedded_io_async::Read + 'static,
    TX: embedded_io_async::Write + 'static,
    WORLD: World,
    SM: RawMutex,
> MCClient<'a, M, RX, TX, WORLD, SM>
where
    RX::Error: 'static,
    TX::Error: 'static,
{
    pub fn new(
        server: &'a MCServer<'a, SM, WORLD>,
        rx: RX,
        tx: TX,
        addr: Option<SocketAddr>,
    ) -> Self {
        Self {
            server,
            rx: Mutex::new(rx),
            tx: Mutex::new(tx),
            addr,
            player_data: None,
        }
    }

    async fn handle_handshake(&mut self) -> Result<ClientIntent, MCClientError> {
        let rx = &mut *self.rx.lock().await;
        let (_, packet_type) = self.read_mc_packet_header(rx).await?;
        match packet_type {
            0 => {
                let protocol_version = rx.read_varint::<i32>().await?;
                let server_address = rx.read_utf8().await?;
                let server_port = rx.read_be::<u16>().await?;
                let intent = rx.read_varint::<i32>().await?;
                debug!(
                    "{} handshake: {:?}, {:?}, {:?}, {:?}",
                    self, protocol_version, server_address, server_port, intent
                );
                match intent {
                    1 => Ok(ClientIntent::Status),
                    2 => Ok(ClientIntent::Login),
                    3 => Ok(ClientIntent::Login), // TODO: transfer login?
                    _ => Err(MCClientError::ProtocolError(format!(
                        "Handshaking: invalid intent id: {intent}."
                    ))),
                }
            }
            _ => Err(MCClientError::ProtocolError(format!(
                "Received invalid packet of type {packet_type} before handshake."
            ))),
        }
    }

    async fn handle_status_intent(&mut self) -> Result<(), MCClientError> {
        let rx = &mut *self.rx.lock().await;
        loop {
            let (packet_length, packet_type) = self.read_mc_packet_header(rx).await?;
            match packet_type {
                0 => {
                    // minecraft:status_request
                    debug!("{} status request", self);
                    let mut pkt = MCPacketBuffer::new(0).await;
                    pkt.write_utf8(r#"{"version":{"name":"1.21.8","protocol":772}, "description":{"text":"Hello, world!"}}"#).await?;
                    self.write_mc_packet(pkt).await?;
                }
                1 => {
                    // minecraft:ping_request
                    debug!("{} ping request", self);
                    let timestamp = rx.read_be::<i64>().await?;
                    let mut pkt = MCPacketBuffer::new(1).await;
                    pkt.write_be(timestamp).await?;
                    self.write_mc_packet(pkt).await?;
                }
                _ => {
                    self.skip_unknown_packet(&mut *rx, packet_type, packet_length)
                        .await?;
                }
            }
        }
    }

    async fn handle_login(&mut self) -> Result<(), MCClientError> {
        let rx = &mut *self.rx.lock().await;
        loop {
            let (packet_length, packet_type) = self.read_mc_packet_header(rx).await?;
            match packet_type {
                0 => {
                    // login start (minecraft:hello)
                    debug!("{} login start", self);
                    {
                        let player_name = rx.read_utf8().await?;
                        let _given_player_uuid = rx.read_uuid().await?;
                        let player_uuid = Uuid::new_mc_offline_player(&player_name);

                        self.player_data = Some(PlayerData {
                            uuid: player_uuid,
                            name: player_name,
                        });
                    }

                    let mut pkt = MCPacketBuffer::new(2).await; // minecraft:login_finished
                    pkt.write_uuid(self.player_data.as_ref().unwrap().uuid)
                        .await?;
                    pkt.write_utf8(&self.player_data.as_ref().unwrap().name)
                        .await?;
                    pkt.write_varint::<u32>(0).await?; // empty properties array
                    self.write_mc_packet(pkt).await?;
                }
                3 => {
                    // minecraft:login_acknowledged
                    debug!("{} login acknowledged", self);
                    return Ok(());
                }
                _ => {
                    self.skip_unknown_packet(&mut *rx, packet_type, packet_length)
                        .await?;
                }
            }
        }
    }

    async fn handle_configure(&mut self) -> Result<(), MCClientError> {
        let mut pkt = MCPacketBuffer::new(14).await; // minecraft:select_known_packs
        pkt.write_varint(1u32).await?;
        pkt.write_utf8("minecraft").await?;
        pkt.write_utf8("core").await?;
        pkt.write_utf8("1.21.8").await?;
        self.write_mc_packet(pkt).await?;

        // let mut pkt = MCPacketBuffer::new(7).await; // minecraft:registry_data
        //
        // self.socket.write_mc_packet(pkt).await?;

        {
            self.tx
                .lock()
                .await
                .write_all(&REGISTRY_PACKETS_DATA)
                .await
                .map_err(EIOError::from)?;
        }

        self.write_mc_packet(MCPacketBuffer::new(3).await).await?; // minecraft:finish_configuration

        let rx = &mut *self.rx.lock().await;
        loop {
            let (packet_length, packet_type) = self.read_mc_packet_header(rx).await?;
            match packet_type {
                3 => {
                    // minecraft:finish_configuration
                    debug!("{} finish configuration", self);
                    return Ok(());
                }
                _ => {
                    self.skip_unknown_packet(&mut *rx, packet_type, packet_length)
                        .await?;
                }
            }
        }
    }

    async fn play_keep_alive(&self) -> Result<(), MCClientError> {
        let mut ticker = Ticker::every(Duration::from_secs(5));
        loop {
            let mut pkt = MCPacketBuffer::new(38).await;
            pkt.write_be(0u64).await?;
            self.write_mc_packet(pkt).await?;
            ticker.next().await;
        }
    }

    async fn play_handle_packets(&self) -> Result<(), MCClientError> {
        let rx = &mut *self.rx.lock().await;
        loop {
            let (packet_length, packet_type) = self.read_mc_packet_header(rx).await?;
            match packet_type {
                _ => {
                    self.skip_unknown_packet(rx, packet_type, packet_length)
                        .await?;
                }
            }
        }
    }

    async fn play(&self) -> Result<(), MCClientError> {
        let result =
            embassy_futures::select::select(self.play_handle_packets(), self.play_keep_alive())
                .await;

        match result {
            Either::First(it) => it,
            Either::Second(it) => it,
        }
    }

    pub async fn run(mut self) -> Result<(), MCClientError> {
        match self.handle_handshake().await? {
            ClientIntent::Status => self.handle_status_intent().await?,
            ClientIntent::Login => {
                self.handle_login().await?;
                self.handle_configure().await?;

                // self.server.add_player(Box::new(self)).await;
                unsafe {
                    self.server
                        .add_player(unsafe { &*(&self as *const Self) })
                        .await
                };

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
                self.write_mc_packet(pkt).await?;

                let mut pkt = MCPacketBuffer::new(34).await; // minecraft:game_event
                pkt.write_be::<u8>(13).await?; // Start waiting for level chunks
                pkt.write_be::<f32>(0.0).await?;
                self.write_mc_packet(pkt).await?;

                let mut pkt = MCPacketBuffer::new(65).await; // minecraft:player_position
                pkt.write_varint::<u32>(0).await?;
                pkt.write_be::<f64>(0.0).await?;
                pkt.write_be::<f64>(100.0).await?;
                pkt.write_be::<f64>(0.0).await?;
                pkt.write_be::<f64>(0.0).await?;
                pkt.write_be::<f64>(0.0).await?;
                pkt.write_be::<f64>(0.0).await?;
                pkt.write_be::<f32>(0.0).await?;
                pkt.write_be::<f32>(0.0).await?;
                pkt.write_be::<u32>(0).await?;
                self.write_mc_packet(pkt).await?;

                let mut pkt = MCPacketBuffer::new(87).await; // set_chunk_cache_center
                pkt.write_varint(0u32).await?;
                pkt.write_varint(0u32).await?;
                self.write_mc_packet(pkt).await?;

                // for cx in -2i16..=2 {
                //     for cz in -2i16..=2 {
                //
                //     }
                // }

                for cx in -2i16..=2 {
                    for cz in -2i16..=2 {
                        self.send_chunk(ChunkPos::new(cx, cz)).await;
                    }
                }

                self.play().await?;

                self.server.remove_player(self.uuid()).await;
            }
        };
        Ok(())

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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[maybe_derive_format]
enum ClientIntent {
    Status,
    Login,
}

#[derive(Debug, derive_more::Display)]
#[display("{self:?}")]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MCClientError {
    /// Logical protocol error (e.g. wrong packet sequence)
    ProtocolError(String),
    /// Packet data format error (e.g. varint too big)
    DataError(#[cfg_attr(feature = "defmt", defmt(Debug2Format))] Box<dyn Error>),
    NetworkError(#[cfg_attr(feature = "defmt", defmt(Debug2Format))] Box<dyn Error>),
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
