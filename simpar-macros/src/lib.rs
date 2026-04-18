#![cfg(feature = "macros")]
//! Procedural macro for declarative string parsing.

mod parse;

use crate::parse::parse_impl;
use proc_macro::TokenStream;

/// Declarative string parser macro.
///
/// The `parse!` macro takes input (string or identifier) and a pattern:
///
/// ```ignore
/// parse!(input -> pattern)
/// ```
///
/// Pattern syntax:
/// - `var` or `var:Type` - capture variable
/// - `_` - blank (skip)
/// - `(pattern)*sep` - repetition with separator
/// - Separators: `,` (space), `;` (line), `#` (block), `~` (multispace)
///
/// ## Examples
///
/// ```
/// let (name, age: u32) = parse!("Alice 30" -> name, age: u32);
/// let nums = parse!("1 2 3" -> (n: i32)*,);
/// ```
#[proc_macro]
pub fn parse(item: TokenStream) -> TokenStream {
    parse_impl(item)
}
