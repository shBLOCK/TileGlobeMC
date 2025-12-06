use alloc::collections::BTreeMap;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use dynify::Dynify;
use smallvec::SmallVec;
use uuid::Uuid;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::mutex::Mutex;
use tileglobe::world::world::World;
use crate::player::DynifiedPlayer;

pub struct MCServer<'a, M: RawMutex, WORLD: World> {
    pub world: &'a WORLD,
    _phantom: PhantomData<M>,
    players: Mutex<M, BTreeMap<Uuid, &'a dyn DynifiedPlayer>>,
    // players: Mutex<M, BTreeMap<Uuid, Arc<dyn DynifiedPlayer>>>,
}
impl<'a, M: RawMutex, WORLD: World> MCServer<'a, M, WORLD> {
    pub fn new(world: &'a WORLD) -> Self {
        Self {
            world,
            // players: Mutex::new(BTreeMap::new()),
            _phantom: Default::default(),
            players: Mutex::new(BTreeMap::new()),
        }
    }

    pub async unsafe fn add_player(&self, player: &'a impl DynifiedPlayer) {
        let mut c = SmallVec::<[MaybeUninit<u8>; 64]>::new();
        self.players.lock().await.insert(player.uuid().init(&mut c).await, player);
    }

    pub async fn remove_player(&self, uuid: Uuid) {
        self.players.lock().await.remove(&uuid);
    }

    // pub async fn add_player<'s, T: DynifiedPlayer + 'static>(
    //     &'s self,
    //     player: T,
    // ) -> Result<&'s dyn DynifiedPlayer, ()> {
    //     let uuid = player.uuid();
    //     let mut players = self.players.lock().await;
    //     if players.contains_key(&uuid) {
    //         return Err(());
    //     }
    //     players.insert(uuid, Box::new(player));
    //     players.map()
    // }

    // pub async fn remove_player(&self, uuid: Uuid) -> Option<Box<dyn DynifiedPlayer>> {
    //     self.players.lock().await.remove(&uuid)
    // }

    pub async fn player_use_item_on() {}

    // pub async fn run(&mut self) {
    //     loop {
    //         self.world.tick().await;
    //     }
    // }

    pub async fn tick(&self) {
        let mut c = SmallVec::<[MaybeUninit<u8>; 64]>::new();

        let block_update_packets = self.world.gen_blocks_update_packets_and_clear_changes().await;
        for player in self.players.lock().await.values() {
            for pkt in &block_update_packets {
                let _ = player.send_mc_packet(pkt).init(&mut c).await;
            }
        }

        for player in self.players.lock().await.values() {
            player.tick().init(&mut c).await
        }
    }
}
