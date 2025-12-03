use core::marker::PhantomData;
use embassy_sync::blocking_mutex::raw::RawMutex;
use tileglobe::world::world::World;

pub struct MCServer<'a, M: RawMutex, WORLD: World> {
    world: &'a mut WORLD,
    _phantom: PhantomData<M>,
    // players: Mutex<M, BTreeMap<Uuid, Arc<dyn DynifiedPlayer>>>,
}
impl<'a, M: RawMutex, WORLD: World> MCServer<'a, M, WORLD> {
    pub fn new(world: &'a mut WORLD) -> Self {
        Self {
            world,
            // players: Mutex::new(BTreeMap::new()),
            _phantom: Default::default(),
        }
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
}
