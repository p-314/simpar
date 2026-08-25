use simpar::parse;

fn parse(input: &str) -> (Vec<usize>, impl Iterator<Item = Vec<(usize, usize, usize)>>) {
    parse!(input -> _, [seeds: usize],* # (_; [maps: usize, $maps: usize, $maps: usize];*)#*);
    (seeds, maps)
}

#[cfg_attr(test, test)]
fn main() {
    let input = include_str!("input.txt");

    let (seeds, maps) = parse(input);

    dbg!(&seeds);
    assert_eq!(seeds, vec![79, 14, 55, 13]);

    let map_vec = maps.collect::<Vec<_>>();
    dbg!(&map_vec);
    assert_eq!(
        map_vec,
        vec![
            vec![(50, 98, 2), (52, 50, 48)],
            vec![(0, 15, 37), (37, 52, 2), (39, 0, 15)],
            vec![(49, 53, 8), (0, 11, 42), (42, 0, 7), (57, 7, 4)],
            vec![(88, 18, 7), (18, 25, 70)],
            vec![(45, 77, 23), (81, 45, 19), (68, 64, 13)],
            vec![(0, 69, 1), (1, 0, 69)],
            vec![(60, 56, 37), (56, 93, 4)]
        ]
    );
}
