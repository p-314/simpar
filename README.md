# simpar

A simple declarative string parser using string operations from the standard library.

The `parse!` macro allows you to extract variables from strings based on specified
patterns, with support for type conversion and various separators.

## Basic Usage
The `parse!` macro takes an input and a pattern of identifiers. Seperators can be used
to split the input. 

```
use simpar::parse;

parse!("hello world" -> x, y);
assert_eq!(x, "hello");
assert_eq!(y, "world");
```

## Type Annotations

Specify types to automatically convert extracted values using the `FromStr` trait:

```
use simpar::parse;

parse!("42 3.14" -> count: u32, ratio: f64);
assert_eq!(count, 42);
assert_eq!(ratio, 3.14);
```

## Separators

Control how patterns are delimited using different separators:

```
use simpar::parse;

// Space separator (`,`)
parse!("hello world" -> a, b);
assert_eq!(a, "hello");
assert_eq!(b, "world");

// Newline separator (`;`)
parse!("first\nsecond" -> line1; line2);
assert_eq!(line1, "first");
assert_eq!(line2, "second");

// Block separator (`#`)
parse!("block1\n\nblock2" -> block1 # block2);
assert_eq!(block1, "block1");
assert_eq!(block2, "block2");

// Multispace separator (`~`)
parse!("data    value" -> x~ y);
assert_eq!(x, "data");
assert_eq!(y, "value");
```

## Repetitions

Extract repeating patterns using `(pattern)*separator`:

```
use simpar::parse;

// Collect space-separated values
parse!("1 2 3 4" -> (n: i32)*,);
let collected: Vec<i32> = n.collect();
assert_eq!(collected, vec![1, 2, 3, 4]);
```

## Complex Patterns

Combine features for flexible parsing:

```
use simpar::parse;

// Skip fields with `_`, mix separators
parse!("Alice _ 30" -> name, _, age: u32);
assert_eq!(age, 30);

// Block separator for multi-line blocks
let text = "Hello\n\nWorld!";

parse!(text -> (mut block)*#);
assert_eq!(block.next(), Some("Hello"));
assert_eq!(block.next(), Some("World!"));
```

## Pattern Syntax Reference

- `var` - capture as string
- `var: Type` - capture and convert to type
- `_` - blank (skip)
- `(pattern)*,` - repetition with space separator
- `(pattern)*;` - repetition with newline separator
- `(pattern)*#` - repetition with block separator
- `(pattern)*~` - repetition with multispace separator