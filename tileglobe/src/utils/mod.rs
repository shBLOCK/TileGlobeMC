pub trait NumEnumU8<const N: u8>: num_enum::FromPrimitive + From<u8> + Into<u8> {}

pub trait Dynified<D> {
    fn dynified(&self) -> &dyn D;
}

impl<T: Dynified<D> + D, D> Dynified<D> for T {
    fn dynified(&self) -> &dyn D {
        self
    }
}
