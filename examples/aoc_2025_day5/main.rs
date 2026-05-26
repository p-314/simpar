use simpar::parse;

fn parse(input: &str) -> (impl Iterator<Item = (u64, u64)>, impl Iterator<Item = u64>) {
    parse!(input -> (id_ranges)*; # (ids: u64)*;);
    let id_ranges = id_ranges.map(|line| {
        parse!(line -> left: u64 "-" right: u64);
        (left, right)
    });
    (id_ranges, ids)
}

fn main() {
    let input = include_str!("input.txt");

    let (id_ranges, ids) = parse(input);

    assert_eq!(
        id_ranges.collect::<Vec<_>>(),
        vec![(3, 5), (10, 14), (16, 20), (12, 18),]
    );
    assert_eq!(ids.collect::<Vec<_>>(), vec![1, 5, 8, 11, 17, 32,]);
}
