//! # simpar
//!
//! A simple declarative string parser using string operations from the standard library.
//!
//! The `parse!` macro allows you to extract variables from strings based on specified
//! patterns, with support for type conversion and various separators.
//!
//! ## Basic Usage
//!
//! ```
//! use simpar::parse;
//!
//! parse!("hello world" -> x, y);
//! assert_eq!(x, "hello");
//! assert_eq!(y, "world");
//! ```
//!
//! ## Type Annotations
//!
//! Specify types to automatically convert extracted values using the `FromStr` trait:
//!
//! ```
//! use simpar::parse;
//!
//! parse!("42 3.14" -> count: u32, ratio: f64);
//! assert_eq!(count, 42);
//! assert_eq!(ratio, 3.14);
//! ```
//!
//! ## Separators
//!
//! Control how patterns are delimited using different separators:
//!
//! ```
//! use simpar::parse;
//!
//! // Space separator (`,`)
//! parse!("hello world" -> a, b);
//! assert_eq!(a, "hello");
//! assert_eq!(b, "world");
//!
//! // Newline separator (`;`)
//! parse!("first\nsecond" -> line1; line2);
//! assert_eq!(line1, "first");
//! assert_eq!(line2, "second");
//!
//! // Block separator (`#`)
//! parse!("block1\n\nblock2" -> block1# block2);
//! assert_eq!(block1, "block1");
//! assert_eq!(block2, "block2");
//!
//! // Multispace separator (`~`)
//! parse!("data    value" -> x~ y);
//! assert_eq!(x, "data");
//! assert_eq!(y, "value");
//! ```
//!
//! ## Repetitions
//!
//! Extract repeating patterns using `(pattern)*separator`:
//!
//! ```
//! use simpar::parse;
//!
//! // Collect space-separated values
//! parse!("1 2 3 4" -> (n: i32)*,);
//! let collected: Vec<i32> = n.collect();
//! assert_eq!(collected, vec![1, 2, 3, 4]);
//! ```
//!
//! ## Complex Patterns
//!
//! Combine features for flexible parsing:
//!
//! ```
//! use simpar::parse;
//!
//! // Skip fields with `_`, mix separators
//! parse!("Alice _ 30" -> name, _, age: u32);
//! assert_eq!(age, 30);
//!
//! // Block separator for multi-line blocks
//! let text = "Hello\n\nWorld!";
//!
//! parse!(text -> (mut block)*#);
//! assert_eq!(block.next(), Some("Hello"));
//! assert_eq!(block.next(), Some("World!"));
//! ```
//!
//! ## Pattern Syntax Reference
//!
//! - `var` - capture as string
//! - `var: Type` - capture and convert to type
//! - `_` - blank (skip)
//! - `(pattern)*,` - repetition with space separator
//! - `(pattern)*;` - repetition with newline separator
//! - `(pattern)*#` - repetition with block separator
//! - `(pattern)*~` - repetition with multispace separator

pub use simpar_macros::parse;

/// Splits a string at the first newline.
///
/// Returns the part before the newline and the part after (excluding the newline).
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
/// Returns the part before the empty line and the remainder (excluding the empty line).
#[inline]
pub fn split_block(s: &str) -> Option<(&str, &str)> {
    if let Some(empty_line) = s.lines().find(|line| line.is_empty()) {
        let (mut block, mut remainder) = unsafe {
            // SAFETY: `empty_line` is a subslice of `s`
            let i = empty_line.as_ptr().offset_from_unsigned(s.as_ptr());
            // SAFETY: `i` is a valid slice index
            s.split_at_checked(i).unwrap_unchecked()
        };

        block = block.strip_suffix('\n').unwrap_or(block);
        block = block.strip_suffix('\r').unwrap_or(block);

        remainder = remainder.strip_prefix('\r').unwrap_or(remainder);
        remainder = remainder.strip_prefix('\n').unwrap_or(remainder);

        Some((block, remainder))
    } else {
        None
    }
}

/// Splits a string at the first space, trimming leading spaces from the remainder.
///
/// Returns the part before the space and the part after (with leading spaces removed).
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

/// Iterator over text blocks separated by empty lines.
pub struct BlockIter<'a> {
    source: &'a str,
    lines: std::str::Lines<'a>,
}

impl<'a> Iterator for BlockIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next_line) = self.lines.next() {
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
                let mut block = &self.source[start_index..end_index];
                block = block.strip_suffix('\n').unwrap_or(block);
                block = block.strip_suffix('\r').unwrap_or(block);

                Some(block)
            } else {
                Some(&self.source[start_index..])
            }
        } else {
            None
        }
    }
}

/// Provides block iteration over strings.
pub trait BlockIterable {
    /// Returns an iterator over blocks (text separated by empty lines).
    fn blocks<'a>(&'a self) -> BlockIter<'a>;
}

impl BlockIterable for str {
    fn blocks<'a>(&'a self) -> BlockIter<'a> {
        BlockIter {
            source: self,
            lines: self.lines(),
        }
    }
}
