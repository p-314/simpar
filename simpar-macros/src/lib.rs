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
/// use simpar::parse;
/// 
/// parse!("Alice 30" -> name, age: u32);
/// assert_eq!("Alice", name);
/// assert_eq!(30, age);
/// 
/// parse!("1 2 3" -> (mut n: i32)*,);
/// assert_eq!(Some(1), n.next());
/// assert_eq!(Some(2), n.next());
/// assert_eq!(Some(3), n.next());
/// assert_eq!(None, n.next());
/// ```
#[proc_macro]
pub fn parse(item: TokenStream) -> TokenStream {
    parse_impl(item)
}
