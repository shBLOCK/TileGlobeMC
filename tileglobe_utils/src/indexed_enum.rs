use num_traits::{PrimInt, Unsigned};

pub trait IndexedEnum<I: PrimInt + Unsigned>: TryFrom<I> + From<I> + Into<I> + 'static {
    const VARIANTS: &'static [Self];

    fn variant_count() -> I {
        I::from(Self::VARIANTS.len()).unwrap()
    }

    fn variants() -> &'static [Self] {
        &Self::VARIANTS
    }
}