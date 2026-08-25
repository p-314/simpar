use std::fs::read_to_string;

static FILES: [&str; 5] = [
    "src/lib.rs",
    "simpar-macros/src/lib.rs",
    "simpar-macros/src/parse.rs",
    "tests/test.rs",
    "tests/format.rs",
];

#[test]
fn no_todos() {
    for file in FILES {
        let source = read_to_string(file).unwrap();

        for (i, line) in source.lines().enumerate() {
            if line.to_lowercase().contains("todo") {
                panic!("todo on line {} in `{}`", i, file);
            }
        }
    }
}
