# simpar

[![crates.io](https://img.shields.io/crates/v/simpar)](https://crates.io/crates/simpar)
[![docs.rs](https://img.shields.io/docsrs/simpar)](https://docs.rs/simpar/latest/simpar/)
![Crates.io License](https://img.shields.io/crates/l/simpar)

A simple declarative string parser using string operations from the standard library.

The [`parse!`](https://docs.rs/simpar/latest/simpar/macro.parse.html) macro allows you to extract variables from strings based on specified
patterns, with support for type conversion and various separators.

For example, if `s` is a string of the form `"<name> <age> birthday: <day>.<month>.<year>"`
then name, age and the birthday can be retrieved with:

```rust
use simpar::parse;

let s = "Alice 42 birthday: 1.1.1970";

parse!(s -> name, age: u8, _, day.month.year);

assert_eq!(name, "Alice");
assert_eq!(age, 42);
assert_eq!((day, month, year), ("1", "1", "1970"));
```


## Pattern Syntax Reference
The `parse!` macro takes input (e.g. a string or identifier) and a pattern:

```rust
parse!(input -> pattern);
```

A pattern consists of matches (usually identifiers) followed by separators. Valid
matches are:

- `<var>` - capture as string slice and assign it to `<var>`
- `<var>: <type>` - capture and convert to type
- `$<var>` - reference match, combining captures into a tuple `(<var>, $<var>)`
- `_` - blank (skip)
- `(<pattern>)<sep>*` - repetition where `<sep>` can be any valid separator
- `[<pattern>]<sep>*` - repetition collected into a `Vec`

Supported separators are:

|separator|symbol|splits at|<div style="width:20em">example</div>|
|----|:--:|----|----|
| Space | `,` | whitespace (`' '`)  | `parse!("AA BBB" -> a, b)` |
| Newline | `;` | newline (`'\n'` or `"\r\n"`)  | `parse!("AA\nBBB" -> a; b)` |
| Paragraph | `#` | empty line | `parse!("AA\n\nBBB" -> a # b)` |
| Period | `.` | period (`'.'`) | `parse!("AA.BBB" -> a. b)` |
| Literal | literal char or string | next occurrence of the literal | `parse!("AAxBBB" -> a "x" b)` |
| ByteOffset | `[+i]` with an integer literal `i` or expression | byte index `i` | `parse!("AABBB" -> a [+2] b)` |

## Type Annotations
By using `<var>: <type>` values are automatically converted using the `FromStr` trait.
The `Result` is unwrapped by default. Using `<var>: <type>?` instead returns the `Result`
and does not panic.

```rust
use simpar::parse;

parse!("42 3.14" -> count: u32, ratio: f64?);
assert_eq!(count, 42);
assert_eq!(ratio, Ok(3.14));
```

## Repetitions
Repeating patterns can be extracted using `(<pattern>)<separator>*`:

```rust
use simpar::parse;

parse!("1 2 3 4" -> (mut n: i32),*);

assert_eq!(n.next(), Some(1));
assert_eq!(n.next(), Some(2));
assert_eq!(n.next(), Some(3));
assert_eq!(n.next(), Some(4));
assert_eq!(n.next(), None);
```

Repetitions return iterators, but can be directly collected into vectors using
the `[<pattern>]<separator>*` syntax.


```rust
use simpar::parse;

parse!("1 2 3 4" -> [n: i32],*);

assert_eq!(n, vec![1, 2, 3, 4]);
```

Multiple variables in repetitions create multiple separate iterators.

## Programmable separators
Some separators can be modified. `{<separator> = <pattern>}` sets the separator to `<pattern>`
where `<pattern>` can be anything that implements the standard library `Pattern` trait,
e.g. a string or char.

For example, if `file` is the content of a CSV file like

```csv
country,capital,population,top-level domain
germany,Berlin,83497147,.de
```

then parsing can be done with:

```rust
parse!(file -> _; {, = ','} country, capital, population: u64, tld);
```

Only the space (`,`) and period (`.`) separator are programmable.

## Condensing
By default every separator splits exactly once. Using `<separator>~` changes that behavior to
return the first remainder that is not empty.

For example `,~` splits the input at consecutive spaces.

```rust
parse!("long      pause" -> x,~ y);

assert_eq!(x, "long");
assert_eq!(y, "pause");
```

## Reference Matches
Prefixing a variable name with `$` (e.g. `$<var>`) creates a reference match.
Reference matches capture additional values for a previously introduced variable `<var>`
and combine all captures for that variable into a tuple `(<var>, $<var>)`. They preserve
the original capture order in the resulting tuple.

```rust
parse!("hello world!" -> a, $a);

assert_eq!(a, ("hello", "world!"));
```

When combined with repetitions or vector collection, reference matches aggregate values 
alongside the initial capture:

```rust
parse!("1-10 14-16 101-102" -> [ranges: usize "-" $ranges: usize],*);

assert_eq!(ranges, vec![(1, 10), (14, 16), (101, 102)]);
```
References can also use a zero-based capture index:

```rust
parse!("zero one two three" -> a, b, c, $2);
assert_eq!(c, ("two", "three"));
```

# License
Simpar is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-MIT](LICENSE-MIT) or [LICENSE-APACHE](LICENSE-APACHE) for more details.
