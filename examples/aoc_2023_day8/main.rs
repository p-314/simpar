use simpar::parse;

fn parse(input: &str) -> (&str, impl Iterator<Item = (&str, (&str, &str))>) {
    parse!(input -> directions # (lines)*;);
    let map = lines.map(|line| {
        parse!(line -> pos " = (" left ", " right ")");
        (pos, (left, right))
    });
    (directions, map)
}

fn main() {
    let input = include_str!("input.txt");

    let (dir, map) = parse(input);

    assert_eq!(dir, "RL");
    assert_eq!(
        map.collect::<Vec<_>>(),
        vec![
            ("AAA", ("BBB", "CCC")),
            ("BBB", ("DDD", "EEE")),
            ("CCC", ("ZZZ", "GGG")),
            ("DDD", ("DDD", "DDD")),
            ("EEE", ("EEE", "EEE")),
            ("GGG", ("GGG", "GGG")),
            ("ZZZ", ("ZZZ", "ZZZ"))
        ]
    );
}
