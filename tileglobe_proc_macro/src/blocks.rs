#![allow(unused)]

use crate::utils::{read_json, resloc_path};
use std::str::FromStr;
use tileglobe_utils::resloc::ResLoc;

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

pub fn read_block_def(resloc: ResLoc) -> BlockDef {
    serde_json::from_value(read_json(resloc_path("blocks", resloc, "json"))).unwrap()
}

pub mod macros {
    use super::*;
    use proc_macro::TokenStream;
    use proc_macro2::Literal;
    use quote::quote;
    use quote::ToTokens;
    use syn::{parse_macro_input, LitStr};

    pub fn mc_block_id_base(input: TokenStream) -> TokenStream {
        let input = parse_macro_input!(input as LitStr).value();
        let block = read_block_def(ResLoc::try_from(input.as_str()).unwrap());
        let id_base = block.id_base;
        Literal::i32_unsuffixed(id_base).into_token_stream().into()
    }
}