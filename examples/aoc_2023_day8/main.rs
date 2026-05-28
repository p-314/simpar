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

    dbg!(dir);
    assert_eq!(dir, "RL");

    let map_vec = map.collect::<Vec<_>>();
    dbg!(&map_vec);
    assert_eq!(
        map_vec,
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
