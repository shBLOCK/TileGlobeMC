use proc_macro::TokenStream;

mod blocks;
mod utils;

#[proc_macro]
pub fn mc_block_id_base(input: TokenStream) -> TokenStream {
    blocks::macros::mc_block_id_base(input)
}