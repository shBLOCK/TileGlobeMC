#![allow(unused)]

use crate::utils::{list_resloc_files_in_dir, read_json, resloc_path};
use std::convert::{AsRef, Into};
use std::error::Error;
use std::str::FromStr;
use tileglobe_utils::MINECRAFT;
use tileglobe_utils::resloc::ResLoc;

#[derive(Debug, serde::Deserialize)]
pub struct BlockDef {
    #[serde(rename = "resource_location")]
    #[serde(deserialize_with = "ResLoc::de_owned")]
    resloc: ResLoc<'static>,
    default_state: u32,
    id_base: u32,
    total_states: u32,
    #[serde(rename = "blockstate_properties")]
    properties: Vec<Property>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PropertyDef {
    name: String,
    id_group_size: u32,
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

impl BlockDef {
    pub const PATH: &'static str = "block_def";

    pub fn load(resloc: &ResLoc) -> Result<Self, Box<dyn Error>> {
        Ok(serde_json::from_value(read_json(resloc_path(
            Self::PATH,
            resloc,
            "json",
        ))?)?)
    }

    pub fn all_reslocs() -> impl Iterator<Item = ResLoc<'static>> {
        list_resloc_files_in_dir(Self::PATH).map(|e| e.0)
    }

    pub fn load_all() -> impl Iterator<Item = Self> {
        Self::all_reslocs().map(|resloc| {
            Self::load(&resloc)
                .map_err(|err| format!("Failed to load BlockDef of {resloc}: {err}"))
                .unwrap()
        })
    }
}

pub mod macros {
    use super::*;
    use crate::utils::{ParseIdent, list_resloc_files_in_dir};
    use itertools::Itertools;
    use proc_macro::TokenStream;
    use proc_macro2::Literal;
    use quote::quote;
    use quote::{ToTokens, format_ident};
    use std::collections::{HashMap, HashSet};
    use syn::parse::discouraged::AnyDelimiter;
    use syn::parse::{Parse, ParseStream};
    use syn::{LitStr, Token};

    pub fn mc_block_id_base(input: TokenStream) -> TokenStream {
        let input = syn::parse_macro_input!(input as LitStr).value();
        let block = BlockDef::load(&ResLoc::try_from(input.as_str()).unwrap()).unwrap();
        let id_base = block.id_base;
        Literal::u32_unsuffixed(id_base).into_token_stream().into()
    }

    fn resloc_const_ident(resloc: &ResLoc) -> String {
        if resloc.namespace != MINECRAFT {
            format!(
                "{}_{}",
                resloc.namespace.to_ascii_uppercase(),
                resloc.path.to_ascii_uppercase()
            )
        } else {
            format!("{}", resloc.path.to_ascii_uppercase())
        }
    }

    pub fn mc_block_resloc_consts(_input: TokenStream) -> TokenStream {
        let lines = BlockDef::all_reslocs().map(|resloc| {
            let name = format_ident!("{}", resloc_const_ident(&resloc));
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

    pub fn mc_blocks_registry(input: TokenStream) -> TokenStream {
        struct BlocksRegistryEntry {
            block: BlockDef,
            obj_expr: syn::Expr,
        }

        impl Parse for BlocksRegistryEntry {
            fn parse(input: ParseStream) -> syn::Result<Self> {
                let resloc = ResLoc::try_from(input.parse::<syn::LitStr>()?.value().as_str())
                    .unwrap()
                    .into_owned();
                let block = BlockDef::load(&resloc).unwrap();
                input.parse::<Token![=>]>()?;
                let obj_expr = input.parse::<syn::Expr>()?;
                Ok(Self { block, obj_expr })
            }
        }

        struct Input {
            generic_block: syn::Path,
            resloc_consts: syn::Path,
            entries: Vec<BlocksRegistryEntry>,
        }

        impl Parse for Input {
            fn parse(input: ParseStream) -> syn::Result<Self> {
                input.parse_ident("generic_block")?;
                input.parse::<Token![:]>()?;
                let generic_block = input.parse::<syn::Path>()?;
                input.parse::<Token![,]>()?;

                input.parse_ident("resloc_consts")?;
                input.parse::<Token![:]>()?;
                let resloc_consts = input.parse::<syn::Path>()?;
                input.parse::<Token![,]>()?;

                input.parse_ident("entries")?;
                input.parse::<Token![:]>()?;
                let input_entries;
                syn::braced!(input_entries in input);
                let entries = input_entries
                    .parse_terminated(BlocksRegistryEntry::parse, Token![,])?
                    .into_iter()
                    .collect::<Vec<_>>();

                Ok(Self {
                    generic_block,
                    resloc_consts,
                    entries,
                })
            }
        }

        let mut input = syn::parse_macro_input!(input as Input);
        input.entries.sort_by_key(|e| e.block.id_base);

        let resloc_to_input_entry = input
            .entries
            .iter()
            .map(|e| (&e.block.resloc, e))
            .collect::<HashMap<_, _>>();

        let elements = BlockDef::load_all()
            .sorted_by_key(|block| block.id_base)
            .map(|block| {
                let expr = if let Some(entry) = resloc_to_input_entry.get(&block.resloc) {
                    entry.obj_expr.to_token_stream()
                } else {
                    let generic_block = &input.generic_block;
                    let resloc_consts = &input.resloc_consts;
                    let const_ident = format_ident!("{}", resloc_const_ident(&block.resloc));
                    let id_base = Literal::u32_unsuffixed(block.id_base);
                    let num_states = Literal::u32_unsuffixed(block.total_states);
                    let default_state = Literal::u32_unsuffixed(block.default_state);
                    quote! {#generic_block {
                        resloc: #resloc_consts::#const_ident,
                        id_base: #id_base,
                        num_states: #num_states,
                        default_state: #default_state.into(),
                    }}
                };
                let id_base = Literal::u32_unsuffixed(block.id_base);
                quote! {(#id_base, #expr)}
            });

        quote! {[#(#elements),*]}.into()
    }
}
