#![allow(unused)]

use crate::utils::{read_json, resloc_path};
use std::convert::{AsRef, Into};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tileglobe_utils::resloc::ResLoc;
use tileglobe_utils::MINECRAFT;

#[derive(Debug, serde::Deserialize)]
pub struct BlockDef {
    #[serde(rename = "resource_location")]
    #[serde(deserialize_with = "ResLoc::de_owned")]
    resloc: ResLoc<'static>,
    default_state: i32,
    id_base: i32,
    total_states: i32,
    #[serde(rename = "blockstate_properties")]
    properties: Vec<Property>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PropertyDef {
    name: String,
    id_group_size: i32,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Property {
    #[serde(rename = "boolean")]
    Boolean {
        #[serde(flatten)]
        def: PropertyDef,
    },
    #[serde(rename = "integer")]
    Integer {
        #[serde(flatten)]
        def: PropertyDef,
        min: i32,
        max: i32,
    },
    #[serde(rename = "enum")]
    Enum {
        #[serde(flatten)]
        def: PropertyDef,
        values: Vec<String>,
    },
}

const BLOCK_DEFS_PATH: &str = "blocks";

pub fn read_block_def(resloc: &ResLoc) -> BlockDef {
    serde_json::from_value(read_json(resloc_path(BLOCK_DEFS_PATH, resloc, "json"))).unwrap()
}

pub mod macros {
    use super::*;
    use crate::utils::list_resloc_files_in_dir;
    use itertools::Itertools;
    use proc_macro::TokenStream;
    use proc_macro2::Literal;
    use quote::quote;
    use quote::{ToTokens, format_ident};
    use syn::{LitStr, parse_macro_input};

    pub fn mc_block_id_base(input: TokenStream) -> TokenStream {
        let input = parse_macro_input!(input as LitStr).value();
        let block = read_block_def(&ResLoc::try_from(input.as_str()).unwrap());
        let id_base = block.id_base;
        Literal::i32_unsuffixed(id_base).into_token_stream().into()
    }

    pub fn mc_block_resloc_consts(_input: TokenStream) -> TokenStream {
        let lines = list_resloc_files_in_dir(BLOCK_DEFS_PATH)
            .map(|e| e.0)
            .sorted_by_cached_key(|resloc| read_block_def(resloc).id_base)
            .map(|resloc| {
                let name = if resloc.namespace != MINECRAFT {
                    format_ident!(
                        "{}_{}",
                        resloc.namespace.to_ascii_uppercase(),
                        resloc.path.to_ascii_uppercase()
                    )
                } else {
                    format_ident!("{}", resloc.path.to_ascii_uppercase())
                };
                let namespace = if resloc.namespace != MINECRAFT {
                    let namespace = resloc.namespace;
                    quote! {namespace}
                } else {
                    quote! {::tileglobe_utils::MINECRAFT}
                };
                let path = resloc.path;
                quote! {pub const #name: &'static ResLoc<'static> = &ResLoc::new(#namespace, #path);}
            });

        quote! {#(#lines)*}.into()
    }

    struct BlocksRegistryEntry {

    }

    pub fn mc_blocks_registry(input: TokenStream) -> TokenStream {
        input.expand_expr()
    }
}
