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

mod input {
    use simpar::parse;

    #[test]
    fn input_field() {
        let s = ("hello", "world");
        parse!(s.0 -> _);
    }

    #[test]
    fn input_index() {
        let s = ["hello", "world"];
        parse!(s[1] -> _);
    }

    #[test]
    fn input_if_expr() {
        #[allow(unused)]
        fn f(b: bool) {
            parse!(if b {"hello"} else {"world"} -> _);
        }
    }

    #[test]
    fn input_block() {
        parse!({println!("hi"); "hello world"} -> _);
    }

    #[test]
    fn input_binary() {
        struct S {}

        impl std::ops::Sub for S {
            type Output = &'static str;

            fn sub(self, _rhs: Self) -> Self::Output {
                "hello world"
            }
        }

        let s = S {};
        let t = S {};
        parse!(s - t -> _);
    }
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
    fn paragraph() {
        parse!("hello\n\nworld" -> a # b);

        assert_eq!("hello", a);
        assert_eq!("world", b);
    }

    #[test]
    fn period() {
        parse!("07.05.2026" -> day.month.year);

        assert_eq!("07", day);
        assert_eq!("05", month);
        assert_eq!("2026", year);
    }

    #[test]
    fn literal_str() {
        parse!("hello123world456!" -> a "123" b "456" c);

        assert_eq!("hello", a);
        assert_eq!("world", b);
        assert_eq!("!", c);
    }

    #[test]
    fn literal_char() {
        parse!("helloöworldö!" -> a 'ö' b 'ö' c);

        assert_eq!("hello", a);
        assert_eq!("world", b);
        assert_eq!("!", c);
    }

    #[test]
    fn byte_offset() {
        parse!("hello world!" -> a [+5] b [+1] c);

        assert_eq!("hello", a);
        assert_eq!(" ", b);
        assert_eq!("world!", c);
    }

    #[test]
    fn byte_offset_expr() {
        let i = 5;
        let f = |x: usize| x * 100 - 99;
        parse!("hello world!" -> a [+i] b [+f(1)] c);

        assert_eq!("hello", a);
        assert_eq!(" ", b);
        assert_eq!("world!", c);
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
    fn missing_paragraph_end() {
        parse!("hello\n\nworld" -> _ # _ #);
    }

    #[test]
    #[should_panic]
    fn missing_literal() {
        parse!("hello world" -> _ "test" _);
    }

    #[test]
    #[should_panic]
    fn byte_offset_overflow() {
        parse!("hi" -> _ [+3]);
    }

    #[test]
    fn implicit_blank() {
        parse!("hello world !" -> a,,b);

        assert_eq!(a, "hello");
        assert_eq!(b, "!");
    }
}

mod programmable {
    use simpar::parse;

    #[test]
    fn change_start() {
        parse!("hello, world, !" -> {. = ", "} a. b. c);

        assert_eq!("hello", a);
        assert_eq!("world", b);
        assert_eq!("!", c);
    }

    #[test]
    fn change_mid() {
        parse!("hello.world,hi" -> a. {. = ","} b. c);

        assert_eq!("hello", a);
        assert_eq!("world", b);
        assert_eq!("hi", c);
    }

    #[test]
    fn change_between() {
        parse!("hello.world,hi" -> a.b {. = ","} . c);

        assert_eq!("hello", a);
        assert_eq!("world", b);
        assert_eq!("hi", c);
    }

    #[test]
    fn change_space() {
        parse!("hello world++!" -> a,b {, = "++"} , c);

        assert_eq!("hello", a);
        assert_eq!("world", b);
        assert_eq!("!", c);
    }

    #[test]
    fn change_char() {
        parse!("hello worldöhi" -> a, {, = 'ö'} b, c);

        assert_eq!("hello", a);
        assert_eq!("world", b);
        assert_eq!("hi", c);
    }

    #[test]
    fn change_iter() {
        parse!("1,2,3" -> {, = ','} (mut a: u8),*);

        assert_eq!(Some(1), a.next());
        assert_eq!(Some(2), a.next());
        assert_eq!(Some(3), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn change_iter_collect() {
        parse!("1,2,3" -> {, = ','} [a: u8],*);

        assert_eq!(vec![1, 2, 3], a);
    }
}

mod condense {
    use simpar::parse;

    #[test]
    fn condense_space() {
        parse!("Hello     World!" -> a,~ b);

        assert_eq!("Hello", a);
        assert_eq!("World!", b);
    }

    #[test]
    fn condense_newline() {
        parse!("Hello\n\n\nWorld!" -> a;~ b);

        assert_eq!("Hello", a);
        assert_eq!("World!", b);
    }

    #[test]
    fn condense_paragraph() {
        parse!("Hello\n\n\r\n\r\nWorld!" -> a #~ b);

        assert_eq!("Hello", a);
        assert_eq!("World!", b);
    }

    #[test]
    fn condense_period() {
        parse!("07....05...2026" -> a.~ b.~ c);

        assert_eq!("07", a);
        assert_eq!("05", b);
        assert_eq!("2026", c);
    }

    #[test]
    fn condense_literal_str() {
        parse!("hello123123123world456456!" -> a "123"~ b "456"~ c);

        assert_eq!("hello", a);
        assert_eq!("world", b);
        assert_eq!("!", c);
    }

    #[test]
    fn condense_literal_char() {
        parse!("helloööööworldööö!" -> a 'ö'~ b 'ö'~ c);

        assert_eq!("hello", a);
        assert_eq!("world", b);
        assert_eq!("!", c);
    }

    #[test]
    fn condense_byte_offset() {
        parse!("HelloWorld!" -> a[+5]~ b);

        assert_eq!("Hello", a);
        assert_eq!("World!", b);
    }

    #[test]
    fn condense_newline_iter() {
        parse!("hello\nworld\n\n\n!" -> (mut a);~*);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    mod migration {
        use simpar::parse;

        #[test]
        fn multispace() {
            // multispace
            parse!("hello      world" -> a,~ b);

            assert_eq!("hello", a);
            assert_eq!("world", b);

            // iter_multispace
            parse!("hello       world    !" -> (mut a),~*);

            assert_eq!(Some("hello"), a.next());
            assert_eq!(Some("world"), a.next());
            assert_eq!(Some("!"), a.next());
            assert_eq!(None, a.next());
        }

        #[test]
        #[should_panic]
        fn missing_multispace_end() {
            parse!("hello   world" -> _,~ _,~);
        }
    }
}

mod iter {
    use std::any::{type_name, type_name_of_val};

    use simpar::parse;

    #[test]
    fn iter_space() {
        parse!("hello world !" -> (mut a),*);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_newline() {
        parse!("hello\nworld\r\n!" -> (mut a);*);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_paragraphs() {
        parse!("hello\n\nworld\r\n\n!" -> (mut a)#*);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_period() {
        parse!("hello.world.!" -> (mut a).*);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_literal_str() {
        parse!("hello123world123!" -> (mut a)"123"*);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_literal_char() {
        parse!("hello1world1!" -> (mut a)'1'*);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_byte_offset() {
        parse!("helloworld!" -> (mut a)[+5]*);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_between() {
        parse!("test: hello world\r\n\n!" -> _, (mut a),* # b);

        assert_eq!(Some("hello"), a.next());
        assert_eq!(Some("world"), a.next());
        assert_eq!(None, a.next());
        assert_eq!("!", b);
    }

    #[test]
    fn iter_zero_ident() {
        parse!("hello world" -> (_),*);
    }

    #[test]
    fn iter_inside() {
        parse!("Hello world\r\n\n! !" -> (_, mut a)#*);

        assert_eq!(Some("world"), a.next());
        assert_eq!(Some("!"), a.next());
        assert_eq!(None, a.next());
    }

    #[test]
    fn iter_iter() {
        parse!("hello world\n1 2 3" -> ((a),*);*);

        let owned = a.map(|line| line.collect::<Vec<_>>()).collect::<Vec<_>>();
        assert_eq!(vec![vec!["hello", "world"], vec!["1", "2", "3"]], owned);
    }

    #[test]
    fn iter_collect() {
        parse!("hello world !" -> [a],*);

        assert_eq!(type_name_of_val(&a), type_name::<Vec<&str>>());
        assert_eq!(vec!["hello", "world", "!"], a);
    }

    #[test]
    fn iter_collect_byte_offset() {
        parse!("hello world !" -> [a][+5]*);

        assert_eq!(vec!["hello", " worl", "d !"], a);
    }

    #[test]
    #[should_panic]
    fn iter_collect_panic_zero_ident() {
        // parsing should fail, because the last item is too short to split at index 2
        // using just ([+2])*, would not fail, bacause the iterator is not consumed
        parse!("aa bb c" -> [[+2]],*);
    }

    mod multi {
        use simpar::parse;

        #[test]
        fn iter_space_multi() {
            parse!("Hello World !" -> (mut first [+1] mut remainder),*);

            assert_eq!(Some("H"), first.next());
            assert_eq!(Some("W"), first.next());
            assert_eq!(Some("!"), first.next());
            assert_eq!(None, first.next());

            assert_eq!(Some("ello"), remainder.next());
            assert_eq!(Some("orld"), remainder.next());
            assert_eq!(Some(""), remainder.next());
            assert_eq!(None, remainder.next());
        }

        #[test]
        fn iter_space_multi_collect() {
            parse!("Hello World !" -> [first [+1] remainder],*);

            assert_eq!(vec!["H", "W", "!"], first);
            assert_eq!(vec!["ello", "orld", ""], remainder);
        }

        #[test]
        fn iter_iter_multi_collect() {
            parse!("Hello World" -> [first [+1] [lspace]'l'*],*);

            assert_eq!(vec!["H", "W"], first);
            assert_eq!(vec![vec!["e", "", "o"], vec!["or", "d"]], lspace);
        }

        #[test]
        fn iter_multi_change() {
            parse!("1x.2x3 4x.5x6" -> (_ . b {. = "x"} . c),*);

            assert!(["2", "5"].into_iter().eq(b));
            assert!(["3", "6"].into_iter().eq(c));
        }
    }

    mod reference {
        use simpar::parse;

        #[test]
        fn reference() {
            parse!("hello world!" -> a, $a);

            assert_eq!(("hello", "world!"), a);
        }

        #[test]
        fn reference_iter() {
            parse!("Hello world!" -> (mut a [+1] $a),*);

            assert_eq!(Some(("H", "ello")), a.next());
            assert_eq!(Some(("w", "orld!")), a.next());
            assert_eq!(None, a.next());
        }

        #[test]
        fn reference_iter_parse() {
            parse!("1+2 13+14" -> (mut a: u8 '+' $a: usize),*);

            assert_eq!(Some((1u8, 2usize)), a.next());
            assert_eq!(Some((13u8, 14usize)), a.next());
            assert_eq!(None, a.next());
        }

        #[test]
        fn reference_deep() {
            parse!("test Hello world\n! !" -> a, ($a, $a);*);

            let (first, mut iter) = a;
            assert_eq!("test", first);
            assert_eq!(Some(("Hello", "world")), iter.next());
            assert_eq!(Some(("!", "!")), iter.next());
            assert_eq!(None, iter.next());
        }

        #[test]
        fn reference_iter_collect() {
            parse!("Hello world!\ntest" -> [a [+1] $a],*; $a);

            assert_eq!((vec![("H", "ello"), ("w", "orld!")], "test"), a);
        }

        #[test]
        fn reference_iter_multi() {
            parse!("a b-d.123 e-f.42" -> x, [$x "-" y "." [$y: u8][+1]*],*);

            assert_eq!(("a", vec!["b", "e"]), x);
            assert_eq!(vec![("d", vec![1u8, 2, 3]), ("f", vec![4u8, 2])], y);
        }


        #[test]
        fn reference_numbered() {
            parse!("hello world!" -> a, $0);
            assert_eq!(("hello", "world!"), a);

            parse!("hello world !" -> $1, _a, b);
            assert_eq!(("hello", "!"), b);
        
            parse!("a b c d e" -> _a, _b, c, _d, $2);
            assert_eq!(("c", "e"), c);
        }


        #[test]
        fn reference_iter_numbered() {
            parse!("Hello world!" -> (mut a [+1] $0),*);

            assert_eq!(Some(("H", "ello")), a.next());
            assert_eq!(Some(("w", "orld!")), a.next());
            assert_eq!(None, a.next());
        }


        #[test]
        #[should_panic]
        fn reference_compilation_test() {
            parse!("" -> _a, $_a);
            parse!("" -> _a, $_a, $_a, $_a);
            parse!("" -> $_b, _b);
            parse!("" -> _a, $_a, $_b, _b);
            
            parse!("" -> _a, ($_a),*);
            parse!("" -> $_a, (_a),*);
            parse!("" -> (_a),*; $_a);
            parse!("" -> (_a),*; ($_a),*);
            parse!("" -> (_a, $_b),*; ($_a, _b),*);
            
            parse!("" -> ((_a),*; ($_a),*);* # $_a);
            parse!("" -> (($_a),*; (_a),*);* # $_a);
            parse!("" -> ((_a, _b),*; ($_a, $_b),*);* # $_b, $_a);
        }
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
        parse!("1 2 3" -> (mut a: u16),*);

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

    #[test]
    fn parse_result() {
        parse!("2 3.14 test" -> a: u32?, b: f32?, c: usize?);

        assert_eq!(Ok(2u32), a);
        assert_eq!(Ok(3.14f32), b);
        assert!(c.is_err());
    }

    #[test]
    fn parse_result_generic_impl() {
        use std::str::FromStr;

        #[allow(unused)]
        fn f<T: FromStr>(s: &str) -> Result<T, <T as FromStr>::Err> {
            parse!(s -> _, r: T?; _);
            r
        }
    }
}
