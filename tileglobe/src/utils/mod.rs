mod indexed_enum;
pub use indexed_enum::*;

pub trait Dynified<D> {
    fn dynified(&self) -> &dyn D;
}

impl<T: Dynified<D> + D, D> Dynified<D> for T {
    fn dynified(&self) -> &dyn D {
        self
    }
}
