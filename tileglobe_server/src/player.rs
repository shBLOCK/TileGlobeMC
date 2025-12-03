use uuid::Uuid;

#[dynify::dynify(DynifiedPlayer)]
pub trait Player {
    fn uuid(&self) -> Uuid;
}