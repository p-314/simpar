use simpar::parse;

#[test]
fn blank() {
    parse!("hello world" -> _);
}

#[test]
fn one_ident() {
    parse!("hi" -> a);

    assert_eq!("hi", a);
}

mod sep {
    use simpar::parse;

    #[test]
    fn space() {
        parse!("hi mom" -> a, b);

        assert_eq!("hi", a);
        assert_eq!("mom", b);
    }

    #[test]
    fn newline() {
        parse!("hello\nworld" -> a; b);

        assert_eq!("hello", a);
        assert_eq!("world", b);
    }

    #[test]
    fn newline_carriage_return() {
        parse!("hello\r\nworld" -> a; b);

        assert_eq!("hello", a);
        assert_eq!("world", b);
    }

    #[test]
    fn space_and_newline() {
        parse!("hello\nworld !" -> a; b, c);

        assert_eq!("hello", a);
        assert_eq!("world", b);
        assert_eq!("!", c);
    }

    #[test]
    fn multispace() {
        parse!("hello      world" -> a~ b);
        assert_eq!("hello", a);
        assert_eq!("world", b);
    }

    #[test]
    fn block() {
        parse!("hello\n\nworld" -> a # b);
        assert_eq!("hello", a);
        assert_eq!("world", b);    
    }

    #[test]
    #[should_panic]
    fn too_many_ident() {
        parse!("hello world" -> _a, _b, _c);
    }

    #[test]
    #[should_panic]
    fn missing_space_end() {
        parse!("hello world" -> _, _,);
    }

    #[test]
    #[should_panic]
    fn missing_newline_end() {
        parse!("hello\nworld" -> _; _;);
    }

    #[test]
    #[should_panic]
    fn missing_block_end() {
        parse!("hello\n\nworld" -> _ # _ #);
    }

    #[test]
    #[should_panic]
    fn missing_multispace_end() {
        parse!("hello   world" -> _~ _~);
    }
}

mod iter {
    use simpar::parse;

    #[test]
    fn iter_space() {
        parse!("hello world !" -> (mut a)*,);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_newline() {
        parse!("hello\nworld\r\n!" -> (mut a)*;);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_multispace() {
        parse!("hello       world    !" -> (mut a)*~);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_blocks() {
        parse!("hello\n\nworld\r\n\n!" -> (mut a)*#);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_between() {
        parse!("test: hello world\r\n\n!" -> _, (mut a)*,# b);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(None, a.next());
        assert_eq!("!", b);
    }

    #[test]
    fn iter_zero_ident() {
        parse!("hello world" -> (_)*,);
    }

    #[test]
    fn iter_iter() {
        parse!("hello world\n1 2 3" -> ((a)*,)*;);

        let owned = a.map(|line| line.collect::<Vec<_>>()).collect::<Vec<_>>();
        assert_eq!(vec![vec!["hello", "world"], vec!["1", "2", "3"]], owned);
    }
}

mod parse {
    use simpar::parse;

    #[test]
    fn parse() {
        parse!("123" -> a: u32);

        assert_eq!(123u32, a);
    }

    #[test]
    fn iter_parse() {
        parse!("1 2 3" -> (mut a: u16)*,);

        assert_eq!(Some(1u16), a.next());
        assert_eq!(Some(2u16), a.next());
        assert_eq!(Some(3u16), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn parse_generic_impl() {
        use std::fmt::Debug;
        use std::str::FromStr;

        #[allow(unused)]
        fn f<T: FromStr>(s: &str) -> T
        where
            <T as FromStr>::Err: Debug,
        {
            parse!(s -> _, r: T; _);
            r
        }
    }
}

#[test]
fn split_fn() {
    #[inline]
    fn split_line<'a>(s: &'a str) -> (&'a str, &'a str) {
        if let Some(i) = s.find('\n') {
            let (mut line, mut remainder) = s.split_at(i);
            line = line.strip_suffix('\r').unwrap_or(line);
            remainder = remainder.strip_prefix('\n').unwrap_or(remainder);
            (line, remainder)
        } else {
            (s, "")
        }
    }

    #[inline]
    fn split_block<'a>(s: &'a str) -> (&'a str, &'a str) {
        if let Some(empty_line) = s.lines().find(|line| line.is_empty()) {
            let (mut block, mut remainder) = unsafe {
                let i = empty_line.as_ptr().offset_from_unsigned(s.as_ptr());
                s.split_at_checked(i).unwrap_unchecked()
            };

            block = block.strip_suffix('\n').unwrap_or(block);
            block = block.strip_suffix('\r').unwrap_or(block);

            remainder = remainder.strip_prefix('\r').unwrap_or(remainder);
            remainder = remainder.strip_prefix('\n').unwrap_or(remainder);

            (block, remainder)
        } else {
            (s, &s[s.len()..])
        }
    }

    let s = "hi\r\n\r\n";
    let (a, b) = split_block(s);
    assert_eq!(a, "hi");
    assert_eq!(b, "");

    let (a, b) = split_line(s);
    assert_eq!(a, "hi");
    assert_eq!(b, "\r\n");
}
