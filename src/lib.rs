pub use simpar_macros::parse;

// helper functions
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

pub trait BlockIterable {
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
