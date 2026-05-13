//! # simpar
//! 
//! A simple declarative string parser using string operations from the standard library.
//! 
//! The `parse!` macro allows you to extract variables from strings based on specified
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
//! - `_` - blank (skip)
//! - `(<pattern>)*<sep>` - repetition where `<sep>` can be any valid separator
//! - `[<pattern>]*<sep>` - repetition collected into a `Vec`
//! 
//! 
//! Supported separators are:
//! 
//! |separator|symbol|splits at|programmable?|
//! |:---|:--:|----|:--:|
//! | Space | `,` | whitespace (`' '`) | **yes** |
//! | Newline | `;` | newline (`\n` or `\r\n`) | no |
//! | Paragraph | `#` | empty lines | no |
//! | Multispace | `~` | one or more whitespaces (`' '`) | no |
//! | Period | `.` | period (`.`) | **yes** |
//! 
//! 
//! ## Type Annotations
//! By using `<var>: <type>` values are automatically converted using the `FromStr` trait:
//! 
//! ```
//! use simpar::parse;
//! 
//! parse!("42 3.14" -> count: u32, ratio: f64);
//! assert_eq!(count, 42);
//! assert_eq!(ratio, 3.14);
//! ```
//! 
//! The program will panic, if any of the conversions fail.
//! 
//! ## Repetitions
//! 
//! Repeating patterns can be extracted using `(<pattern>)*<separator>`:
//! 
//! ```
//! use simpar::parse;
//! 
//! parse!("1 2 3 4" -> (mut n: i32)*,);
//! 
//! assert_eq!(n.next(), Some(1));
//! assert_eq!(n.next(), Some(2));
//! assert_eq!(n.next(), Some(3));
//! assert_eq!(n.next(), Some(4));
//! assert_eq!(n.next(), None);
//! ```
//! 
//! Repetitions return iterators, but can be directly collected into vectors using
//! the `[<pattern>]*<separator>` syntax.
//! 
//! 
//! ```
//! use simpar::parse;
//! 
//! parse!("1 2 3 4" -> [n: i32]*,);
//! 
//! assert_eq!(n, vec![1, 2, 3, 4]);
//! ```
//! 
//! At the moment repetitions can contain at most one identifier.
//! 
//! ## Programmable separators
//! Some separators can be modified. `{<separator> = <pattern>}` sets the sperator to `<pattern>`
//! where `<pattern>` can be anything that implements the standard library `Pattern` trait, 
//! e.g. a string or char.
//! 
//! For example, if `file` is the content of a CSV file like
//! 
//! ```csv
//! country,capital
//! germany,Berlin
//! ```
//! 
//! then parsing can be done with:
//! 
//! ```
//! # use simpar::parse;
//! # let file = r"country,capital
//! # germany,Berlin";
//! 
//! parse!(file -> _; {, = ','} country, capital);
//! ```


pub use simpar_macros::parse;

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
/// or `None` if the string does not contain an empty line.
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
    if let Some(empty_line) = s.lines().find(|line| line.is_empty()) {
        let (mut paragraph, mut remainder) = unsafe {
            // SAFETY: `empty_line` is a subslice of `s`
            let i = empty_line.as_ptr().offset_from_unsigned(s.as_ptr());
            // SAFETY: `i` is a valid slice index
            s.split_at_checked(i).unwrap_unchecked()
        };

        paragraph = paragraph.strip_suffix('\n').unwrap_or(paragraph);
        paragraph = paragraph.strip_suffix('\r').unwrap_or(paragraph);

        remainder = remainder.strip_prefix('\r').unwrap_or(remainder);
        remainder = remainder.strip_prefix('\n').unwrap_or(remainder);

        Some((paragraph, remainder))
    } else {
        None
    }
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
    lines: std::str::Lines<'a>,
}

impl<'a> Iterator for ParagraphIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next_line) = self.lines.next() {
            // SAFETY: `next_line` and `source` reference the same string and
            // `next_line` is a subslice of `source`
            let start_index = unsafe {
                next_line
                    .as_ptr()
                    .offset_from_unsigned(self.source.as_ptr())
            };
            if let Some(empty_line) = self.lines.find(|line| line.is_empty()) {
                // SAFETY: `empty_line` is a subslice of `source`
                let end_index = unsafe {
                    empty_line
                        .as_ptr()
                        .offset_from_unsigned(self.source.as_ptr())
                };
                let mut paragraph = &self.source[start_index..end_index];
                paragraph = paragraph.strip_suffix('\n').unwrap_or(paragraph);
                paragraph = paragraph.strip_suffix('\r').unwrap_or(paragraph);

                Some(paragraph)
            } else {
                Some(&self.source[start_index..])
            }
        } else {
            None
        }
    }
}

/// Provides paragraph iteration over strings.
pub trait ParagraphIterable {
    /// Returns an iterator over paragraphs (text separated by empty lines).
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
