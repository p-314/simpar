# simpar

![crates.io](https://img.shields.io/crates/v/simpar)
![docs.rs](https://img.shields.io/docsrs/simpar)
![Crates.io License](https://img.shields.io/crates/l/simpar)


A simple declarative string parser using string operations from the standard library.

The `parse!` macro allows you to extract variables from strings based on specified
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
A pattern consists of matches (usually identifiers) followed by separators. Valid 
matches are:

- `<var>` - capture as string slice and assign it to `<var>`
- `<var>: <type>` - capture and convert to type
- `_` - blank (skip)
- `(<pattern>)*<sep>` - repetition where `<sep>` can be any valid separator
- `[<pattern>]*<sep>` - repetition collected into a `Vec`


Supported separators are:

|separator|symbol|splits at|programmable?|
|:---|:--:|----|:--:|
| Space | `,` | whitespace (`' '`) | **yes** |
| Newline | `;` | newline (`'\n'` or `"\r\n"`) | no |
| Paragraph | `#` | empty lines | no |
| Multispace | `~` | one or more whitespaces (`' '`) | no |
| Period | `.` | period (`'.'`) | **yes** |


## Type Annotations
By using `<var>: <type>` values are automatically converted using the `FromStr` trait:

```rust
use simpar::parse;

parse!("42 3.14" -> count: u32, ratio: f64);
assert_eq!(count, 42);
assert_eq!(ratio, 3.14);
```

The program will panic, if any of the conversions fail.

## Repetitions

Repeating patterns can be extracted using `(<pattern>)*<separator>`:

```rust
use simpar::parse;

parse!("1 2 3 4" -> (mut n: i32)*,);

assert_eq!(n.next(), Some(1));
assert_eq!(n.next(), Some(2));
assert_eq!(n.next(), Some(3));
assert_eq!(n.next(), Some(4));
assert_eq!(n.next(), None);
```

Repetitions return iterators, but can be directly collected into vectors using
the `[<pattern>]*<separator>` syntax.


```rust
use simpar::parse;

parse!("1 2 3 4" -> [n: i32]*,);

assert_eq!(n, vec![1, 2, 3, 4]);
```

At the moment repetitions can contain at most one identifier.

## Programmable separators
Some separators can be modified. `{<separator> = <pattern>}` sets the separator to `<pattern>`
where `<pattern>` can be anything that implements the standard library `Pattern` trait, 
e.g. a string or char.

For example, if `file` is the content of a CSV file like

```csv
country,capital
germany,Berlin
```

then parsing can be done with:

```rust
parse!(file -> _; {, = ','} country, capital);
```

# License
Simpar is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-MIT](LICENSE-MIT) or [LICENSE-APACHE](LICENSE-APACHE) for more details.
