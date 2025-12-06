use uuid::Uuid;
use tileglobe_utils::network::MCPacketBuffer;

#[dynify::dynify(DynifiedPlayer)]
pub trait Player {
    async fn uuid(&self) -> Uuid;
    
    async fn tick(&self);

    //TODO: remove this method, let player impl handle the packet logic instead
    async fn send_mc_packet(&self, pkt: &MCPacketBuffer);
}