use simpar::parse;

const LINKS: &str = r"[![crates.io](https://img.shields.io/crates/v/simpar)](https://crates.io/crates/simpar)
[![docs.rs](https://img.shields.io/docsrs/simpar)](https://docs.rs/simpar/latest/simpar/)
![Crates.io License](https://img.shields.io/crates/l/simpar)
";

const LICENSE: &str = r"
# License
Simpar is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-MIT](LICENSE-MIT) or [LICENSE-APACHE](LICENSE-APACHE) for more details.
";

fn format_markdown(input: &str) -> String {
    parse!(input -> [+4] heading; ([+3] lines);* # _);

    let mut result = String::new();
    let mut code_block = false;
    let mut rust_code_block = false;

    result.push_str(heading);
    result.push_str("\n\n");
    result.push_str(LINKS);

    for mut line in lines {
        line = line.trim_start();

        // silent doc test lines
        if rust_code_block {
            if line.starts_with("#") {
                continue;
            }
        }

        result.push_str(line);

        if line.starts_with("```") {
            dbg!(line);
        }

        if line.starts_with("```") {
            code_block = !code_block;
            if line == "```" {
                if code_block {
                    rust_code_block = true;
                    result.push_str("rust");
                } else {
                    rust_code_block = false;
                }
            }
        }

        result.push_str("\n");
    }

    result.push_str(LICENSE);

    result = result.replace(
        "[`parse!`]",
        "[`parse!`](https://docs.rs/simpar/latest/simpar/macro.parse.html)",
    );

    result
}

#[test]
fn readme_formatted() {
    let lib_doc = include_str!("../src/lib.rs");
    let formatted = format_markdown(lib_doc);
    
    println!("{}", formatted);

    assert_eq!(include_str!("../README.md"), formatted.as_str());
}
