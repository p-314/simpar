#![cfg(feature = "macros")]

mod parse;

use proc_macro::TokenStream;
use crate::parse::parse_impl;

/// Macro for parsing a string slice using std library functions.
#[proc_macro]
pub fn parse(item: TokenStream) -> TokenStream {
    parse_impl(item)
}