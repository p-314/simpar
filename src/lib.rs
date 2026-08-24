//! # simpar
//!
//! A simple declarative string parser using string operations from the standard library.
//!
//! The [`parse!`] macro allows you to extract variables from strings based on specified
//! patterns, with support for type conversion and various separators.
//!
//! For example, if `s` is a string of the form `"<name> <age> birthday: <day>.<month>.<year>"`
//! then name, age and the birthday can be retrieved with:
//!
//! ```
//! use simpar::parse;
//!
//! let s = "Alice 42 birthday: 1.1.1970";
//!
//! parse!(s -> name, age: u8, _, day.month.year);
//!
//! assert_eq!(name, "Alice");
//! assert_eq!(age, 42);
//! assert_eq!((day, month, year), ("1", "1", "1970"));
//! ```
//!
//!
//! ## Pattern Syntax Reference
//! The `parse!` macro takes input (e.g. a string or identifier) and a pattern:
//!
//! ```
//! # use simpar_macros::parse;
//! # let input = "";
//! parse!(input -> pattern);
//! ```
//!
//! A pattern consists of matches (usually identifiers) followed by separators. Valid
//! matches are:
//!
//! - `<var>` - capture as string slice and assign it to `<var>`
//! - `<var>: <type>` - capture and convert to type
//! - `$<var>` - reference match, combining captures into a tuple `(<var>, $<var>)`
//! - `_` - blank (skip)
//! - `(<pattern>)<sep>*` - repetition where `<sep>` can be any valid separator
//! - `[<pattern>]<sep>*` - repetition collected into a `Vec`
//!
//! Supported separators are:
//!
//! |separator|symbol|splits at|<div style="width:20em">example</div>|
//! |----|:--:|----|----|
//! | Space | `,` | whitespace (`' '`)  | `parse!("AA BBB" -> a, b)` |
//! | Newline | `;` | newline (`'\n'` or `"\r\n"`)  | `parse!("AA\nBBB" -> a; b)` |
//! | Paragraph | `#` | empty line | `parse!("AA\n\nBBB" -> a # b)` |
//! | Period | `.` | period (`'.'`) | `parse!("AA.BBB" -> a. b)` |
//! | Literal | literal char or string | next occurrence of the literal | `parse!("AAxBBB" -> a "x" b)` |
//! | ByteOffset | `[+i]` with an integer literal `i` or expression | byte index `i` | `parse!("AABBB" -> a [+2] b)` |
//!
//! ## Type Annotations
//! By using `<var>: <type>` values are automatically converted using the `FromStr` trait.
//! The `Result` is unwrapped by default. Using `<var>: <type>?` instead returns the `Result`
//! and does not panic.
//!
//! ```
//! use simpar::parse;
//!
//! parse!("42 3.14" -> count: u32, ratio: f64?);
//! assert_eq!(count, 42);
//! assert_eq!(ratio, Ok(3.14));
//! ```
//!
//! ## Repetitions
//! Repeating patterns can be extracted using `(<pattern>)<separator>*`:
//!
//! ```
//! use simpar::parse;
//!
//! parse!("1 2 3 4" -> (mut n: i32),*);
//!
//! assert_eq!(n.next(), Some(1));
//! assert_eq!(n.next(), Some(2));
//! assert_eq!(n.next(), Some(3));
//! assert_eq!(n.next(), Some(4));
//! assert_eq!(n.next(), None);
//! ```
//!
//! Repetitions return iterators, but can be directly collected into vectors using
//! the `[<pattern>]<separator>*` syntax.
//!
//!
//! ```
//! use simpar::parse;
//!
//! parse!("1 2 3 4" -> [n: i32],*);
//!
//! assert_eq!(n, vec![1, 2, 3, 4]);
//! ```
//!
//! Multiple variables in repetitions create multiple separate iterators.
//!
//! ## Programmable separators
//! Some separators can be modified. `{<separator> = <pattern>}` sets the sperator to `<pattern>`
//! where `<pattern>` can be anything that implements the standard library `Pattern` trait,
//! e.g. a string or char.
//!
//! For example, if `file` is the content of a CSV file like
//!
//! ```csv
//! country,capital,population,top-level domain
//! germany,Berlin,83497147,.de
//! ```
//!
//! then parsing can be done with:
//!
//! ```
//! # use simpar::parse;
//! # let file = r"country,capital,population,top-level domain
//! # germany,Berlin,83497147,.de";
//! #
//! parse!(file -> _; {, = ','} country, capital, population: u64, tld);
//! # assert_eq!(country, "germany");
//! # assert_eq!(capital, "Berlin");
//! # assert_eq!(population, 83497147);
//! # assert_eq!(tld, ".de");
//! ```
//!
//! Only the space (`,`) and period (`.`) seperator are programmable.
//!
//! ## Condensing
//! By default every separator splits exactly once. Using `<separator>~` changes thar behavior to
//! return the first remainder that is not empty.
//!
//! For example `,~` splits the input at consecutive spaces.
//!
//! ```
//! # use simpar::parse;
//! parse!("long      pause" -> x,~ y);
//!
//! assert_eq!(x, "long");
//! assert_eq!(y, "pause");
//! ```
//!
//! ## Reference Matches
//! Prefixing a variable name with `$` (e.g. `$<var>`) creates a reference match.
//! Reference matches capture additional values for a previously introduced variable `<var>`
//! and combine all captures for that variable into a tuple `(<var>, $<var>)`.
//!
//! ```
//! # use simpar::parse;
//! parse!("hello world!" -> a, $a);
//!
//! assert_eq!(a, ("hello", "world!"));
//! ```
//!
//! When combined with repetitions or vector collection, reference matches aggregate values 
//! alongside the initial capture:
//!
//! ```
//! # use simpar::parse;
//! parse!("1-10 14-16 101-102" -> [ranges: usize "-" $ranges: usize],*);
//!
//! assert_eq!(ranges, vec![(1, 10), (14, 16), (101, 102)]);
//! ```

pub use simpar_macros::parse;

use std::str::Lines;

/// Extend a subslice to the right.
///
/// # Panics
/// Panics if `subslice` is not a subslice of `source`.
pub fn subslice_extend_right<'a>(subslice: &str, source: &'a str) -> &'a str {
    let source_ptr = source.as_ptr() as usize;
    let subslice_ptr = subslice.as_ptr() as usize;

    &source[subslice_ptr.checked_sub(source_ptr).unwrap()..]
}

/// Splits a string at the first newline.
///
/// Returns the part before the newline and the part after (excluding the newline)
/// or `None` if the string does not contain a newline.
///
/// # Examples
/// ```
/// use simpar::split_line;
///
/// assert_eq!(Some(("Hello", "world!")), split_line("Hello\nworld!"));
/// assert_eq!(None, split_line("Hello world!"));
/// ```
#[inline]
pub fn split_line(s: &str) -> Option<(&str, &str)> {
    if let Some(i) = s.find('\n') {
        let (mut line, mut remainder) = s.split_at(i);
        line = line.strip_suffix('\r').unwrap_or(line);
        remainder = remainder.strip_prefix('\n').unwrap_or(remainder);
        Some((line, remainder))
    } else {
        None
    }
}

/// Splits a string at the first empty line.
///
/// Returns the part before the empty line and the remainder (excluding the empty line)
/// or `None` if the string does not contain an empty line. <code>a&nbsp;&nbsp;&nbsp;&nbsp; b</code>
///
/// # Examples
/// ```
/// use simpar::split_paragraph;
///
/// assert_eq!(Some(("Hello", "world!")), split_paragraph("Hello\n\nworld!"));
/// assert_eq!(None, split_paragraph("Hello world!"));
/// ```
#[inline]
pub fn split_paragraph(s: &str) -> Option<(&str, &str)> {
    let mut iter = s.paragraphs();
    let paragraph = iter.next()?;
    let remainder = iter.remainder()?;

    Some((paragraph, remainder))
}

/// Splits a string at the first space, trimming leading spaces from the remainder.
///
/// Returns the part before the space and the part after (with leading spaces removed)
/// or `None` if the string does not contain `' '`.
///
/// # Examples
/// ```
/// use simpar::split_multispace;
///
/// assert_eq!(Some(("Hello", "world!")), split_multispace("Hello    world!"));
/// assert_eq!(None, split_multispace("HelloWorld!"));
/// ```
#[inline]
pub fn split_multispace(s: &str) -> Option<(&str, &str)> {
    if let Some(i) = s.find(' ') {
        let (line, mut remainder) = s.split_at(i);
        remainder = remainder.trim_start_matches(' ');
        Some((line, remainder))
    } else {
        None
    }
}

/// Iterator over text paragraphs separated by empty lines.
pub struct ParagraphIter<'a> {
    source: &'a str,
    lines: Lines<'a>,
}

impl<'a> ParagraphIter<'a> {
    /// Consume the iterator and return the remaining string or `None` if
    /// the iterator is empty.
    #[inline]
    pub fn remainder(mut self) -> Option<&'a str> {
        let start_index = unsafe {
            self.next()?
                .as_ptr()
                .offset_from_unsigned(self.source.as_ptr())
        };
        Some(&self.source[start_index..])
    }
}

impl<'a> Iterator for ParagraphIter<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let first_line = self.lines.next()?;
        if first_line.is_empty() {
            return Some(first_line);
        }

        let source_ptr = self.source.as_ptr();
        // SAFETY: `first_line` is a subslice of `source`
        let start_index = unsafe { first_line.as_ptr().offset_from_unsigned(source_ptr) };
        let mut end_index = start_index + first_line.len();

        for next_line in self.lines.by_ref() {
            if next_line.is_empty() {
                break;
            }
            // SAFETY: `next_line` is a subslice of `source`
            end_index =
                unsafe { next_line.as_ptr().offset_from_unsigned(source_ptr) } + next_line.len();
        }

        // `start_index` and `end_index` are both in bounds and on UTF-8 boundarys
        Some(unsafe { self.source.get_unchecked(start_index..end_index) })
    }
}

/// Provides paragraph iteration over strings.
pub trait ParagraphIterable {
    /// Returns an iterator over paragraphs (text separated by exactly one empty line).
    ///
    /// Paragraph terminators are not included in the paragraphs returned by the iterator.
    fn paragraphs<'a>(&'a self) -> ParagraphIter<'a>;
}

impl ParagraphIterable for str {
    fn paragraphs<'a>(&'a self) -> ParagraphIter<'a> {
        ParagraphIter {
            source: self,
            lines: self.lines(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{ParagraphIterable, split_line, split_paragraph};

    #[test]
    fn paragraph() {
        let s = "hello\n\nworld\n\n!";
        let (par, rem) = split_paragraph(s).unwrap();
        assert_eq!(par, "hello");
        assert_eq!(rem, "world\n\n!");
    }

    #[test]
    fn paragraph_carriage_return() {
        let s = "hello\r\n\r\nworld\r\n\n!";
        let (par, rem) = split_paragraph(s).unwrap();
        assert_eq!(par, "hello");
        assert_eq!(rem, "world\r\n\n!");

        let (par, rem) = split_paragraph(rem).unwrap();
        assert_eq!(par, "world");
        assert_eq!(rem, "!");
    }

    #[test]
    fn paragraph_terminator() {
        // ending with an empty paragraph results in no splitting
        let s = "hi\r\n\r\n";
        assert!(split_paragraph(s).is_none());
    }

    #[test]
    fn line() {
        let s = "hello\nworld\n!";
        let (par, rem) = split_line(s).unwrap();
        assert_eq!(par, "hello");
        assert_eq!(rem, "world\n!");
    }

    #[test]
    fn line_carriage_return() {
        let s = "hello\r\nworld\r\n!";
        let (par, rem) = split_line(s).unwrap();
        assert_eq!(par, "hello");
        assert_eq!(rem, "world\r\n!");
    }

    #[test]
    fn line_terminator() {
        // ending with an empty line results in no splitting
        let s = "hi\n";
        assert!(split_paragraph(s).is_none());

        let s = "hi\r\n";
        assert!(split_paragraph(s).is_none());
    }

    #[test]
    fn paragraph_iter() {
        let s = "hi\n\nmom\n\n\n!";
        // hi
        //-
        // mom
        //-
        //-
        // !

        let mut iter = s.paragraphs();
        assert_eq!(iter.next(), Some("hi"));
        assert_eq!(iter.next(), Some("mom"));
        assert_eq!(iter.next(), Some(""));
        assert_eq!(iter.next(), Some("!"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn paragraph_iter_carriage_return() {
        let s = "hi\n\r\nmom\r\n\n\r\n!";
        // hi
        //-
        // mom
        //-
        //-
        // !

        let mut iter = s.paragraphs();
        assert_eq!(iter.next(), Some("hi"));
        assert_eq!(iter.next(), Some("mom"));
        assert_eq!(iter.next(), Some(""));
        assert_eq!(iter.next(), Some("!"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn paragraph_iter_terminator() {
        // `s` ends with two empty lines, but the last one acts as terminator
        // -> only one empty paragraph is returned
        let s = "test\n\n\n";
        // test
        //-
        //-

        let mut iter = s.paragraphs();
        assert_eq!(iter.next(), Some("test"));
        assert_eq!(iter.next(), Some(""));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn paragraph_iter_empty_line_start() {
        // `s` starts with an empty line -> the first paragraph is empty
        let s = "\ntest";
        //-
        // test

        let mut iter = s.paragraphs();
        assert_eq!(iter.next(), Some(""));
        assert_eq!(iter.next(), Some("test"));
        assert_eq!(iter.next(), None);
    }
}
