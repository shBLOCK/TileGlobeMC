use alloc::boxed::Box;
use alloc::collections::LinkedList;
use tileglobe::world::world::World;
use crate::player::Player;

pub struct MCServer<'a, WORLD: World> {
    world: &'a mut WORLD,
    players: LinkedList<Box<dyn Player>>,
}
impl <'a, WORLD: World> MCServer<'a, WORLD> {

}