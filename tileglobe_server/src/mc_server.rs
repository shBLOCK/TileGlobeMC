use alloc::boxed::Box;
use alloc::collections::LinkedList;
use tileglobe::world::world::World;
use crate::player::{DynifiedPlayer, Player};

pub struct MCServer<'a, WORLD: World> {
    world: &'a mut WORLD,
    players: LinkedList<Box<dyn DynifiedPlayer>>,
}
impl <'a, WORLD: World> MCServer<'a, WORLD> {
    pub fn new(world: &'a mut WORLD) -> Self {
        Self {
            world,
            players: LinkedList::new(),
        }
    }

    pub async fn player_use_item_on() {

    }

    pub async fn run(&mut self) {
        loop {
            self.world.tick().await;
        }
    }
}