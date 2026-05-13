#![cfg(feature = "macros")]
//! Simpar procedural macros.
//!
//! # Warning! ⚠️
//!
//! The macros depend on library functions in `simpar` and might
//! not work if imported directly. Use the re-exports in the `simpar` crate
//! instead.

mod parse;

use crate::parse::parse_impl;
use proc_macro::TokenStream;

/// Declarative string parser macro.
///
/// The `parse!` macro takes input (string or identifier) and a pattern:
///
/// ```
/// # use simpar_macros::parse;
/// # let input = "";
/// parse!(input -> pattern)
/// ```
/// A pattern consists of matches (usually identifiers) followed by separators. 
///
/// Match syntax:
/// - `<var>` - capture as string slice and assign it to `<var>`
/// - `<var>: <type>` - capture and convert to type
/// - `_` - blank (skip)
/// - `(<pattern>)*<sep>` - repetition where `<sep>` can be any valid separator
/// - `[<pattern>]*<sep>` - repetition collected into a `Vec`
///
/// Separator Syntax:
///
/// |separator|symbol|splits at|programmable?|
/// |:---|:--:|----|:--:|
/// | Space | `,` | whitespace (`' '`) | **yes** |
/// | Newline | `;` | newline (`\n` or `\r\n`) | no |
/// | Paragraph | `#` | empty lines | no |
/// | Multispace | `~` | one or more whitespaces (`' '`) | no |
/// | Period | `.` | period (`.`) | **yes** |
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
