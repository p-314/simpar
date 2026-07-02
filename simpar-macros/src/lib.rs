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
/// A pattern consists of matches (usually identifiers) followed by separators. Valid
/// matches are:
///
/// - `<var>` - capture as string slice and assign it to `<var>`
/// - `<var>: <type>` - capture and convert to type
/// - `_` - blank (skip)
/// - `(<pattern>)<sep>*` - repetition where `<sep>` can be any valid separator
/// - `[<pattern>]<sep>*` - repetition collected into a `Vec`
///
/// Supported separators are:
///
/// |separator|symbol|splits at|<div style="width:20em">example</div>|
/// |----|:--:|----|----|
/// | Space | `,` | whitespace (`' '`)  | `parse!("AA BBB" -> a, b)` |
/// | Newline | `;` | newline (`'\n'` or `"\r\n"`)  | `parse!("AA\nBBB" -> a; b)` |
/// | Paragraph | `#` | empty line | `parse!("AA\n\nBBB" -> a # b)` |
/// | Period | `.` | period (`'.'`) | `parse!("AA.BBB" -> a. b)` |
/// | Literal | literal char or string | next occurrence of the literal | `parse!("AAxBBB" -> a "x" b)` |
/// | ByteOffset | `[+i]` with an integer literal `i` or expression | byte index `i` | `parse!("AABBB" -> a [+2] b)` |
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
/// parse!("1 2 3" -> (mut n: i32),*);
/// assert_eq!(Some(1), n.next());
/// assert_eq!(Some(2), n.next());
/// assert_eq!(Some(3), n.next());
/// assert_eq!(None, n.next());
/// ```
///
/// For more information, please see the [crate-level documentation](https://docs.rs/simpar/latest/simpar/).
#[proc_macro]
pub fn parse(item: TokenStream) -> TokenStream {
    parse_impl(item)
}
