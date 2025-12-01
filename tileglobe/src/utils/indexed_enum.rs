use num_traits::PrimInt;

pub trait IndexedEnum<I: PrimInt>: TryFrom<I> + From<I> + Into<I> {
    const VARIANTS: &'static [Self];

    fn variant_count() -> I {
        Self::VARIANTS.len() as I
    }

    fn variants() -> &'static [Self] {
        &Self::VARIANTS
    }
}