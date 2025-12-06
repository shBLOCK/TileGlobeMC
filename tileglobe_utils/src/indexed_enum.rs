use num_traits::{NumCast, PrimInt, Unsigned};

pub trait IndexedEnum: TryFrom<Self::Index> + From<Self::Index> + Into<Self::Index> + 'static {
    type Index: PrimInt + Unsigned;
    const VARIANTS: &'static [Self];

    fn variant_count() -> Self::Index {
        <Self::Index as NumCast>::from(Self::VARIANTS.len()).unwrap()
    }

    fn variants() -> &'static [Self] {
        &Self::VARIANTS
    }
}