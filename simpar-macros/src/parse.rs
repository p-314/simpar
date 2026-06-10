use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Expr, Ident, LitChar, LitStr, Token, Type, braced, bracketed, parenthesized, parse_macro_input,
    token::{Brace, Bracket, Paren},
};

struct IdentHelper(&'static str);

macro_rules! new_ident {
    ($name: literal) => {
        IdentHelper(concat!("__simpar_macro_internal_", $name))
    };
}

impl ToTokens for IdentHelper {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        Ident::new(self.0, proc_macro2::Span::call_site()).to_tokens(tokens)
    }
}

const INPUT: IdentHelper = new_ident!("input");
const RETURN_DATA: IdentHelper = new_ident!("return_data");
const ITER: IdentHelper = new_ident!("iter");

#[derive(Clone)]
struct Variable {
    mutability: Option<Token![mut]>,
    ident: Ident,
    // optional conversion type and if the result should be unwrapped
    conversion_type: Option<(Type, bool)>,
}

impl ToTokens for Variable {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            mutability: mu,
            ident: id,
            conversion_type: _,
        } = self.clone();
        tokens.extend(quote! {let #mu #id;});
    }
}

#[derive(Clone)]
enum SplitPattern {
    Str(LitStr),
    Char(LitChar),
    Var(Ident),
}

impl ToTokens for SplitPattern {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            SplitPattern::Str(inner) => inner.to_tokens(tokens),
            SplitPattern::Char(inner) => inner.to_tokens(tokens),
            SplitPattern::Var(inner) => inner.to_tokens(tokens),
        }
    }
}

macro_rules! parse_spat {
    ($input: ident, $format: ident) => {
        let inner;
        braced!(inner in $input);

        let sep = if inner.peek(Token![.]) {
            inner.parse::<Token![.]>()?;
            Separator::Period
        } else if inner.peek(Token![,]) {
            inner.parse::<Token![,]>()?;
            Separator::Space
        } else {
            return Err($input.error("Expected programmable separator (, or .)!"));
        };

        inner.parse::<Token![=]>()?;

        let split_pat = if inner.peek(LitStr) {
            SplitPattern::Str(inner.parse::<LitStr>()?)
        } else if inner.peek(LitChar) {
            SplitPattern::Char(inner.parse::<LitChar>()?)
        } else if inner.peek(Ident) {
            SplitPattern::Var(inner.parse::<Ident>()?)
        } else {
            return Err($input.error("Expected literal or identifier!"));
        };

        let pro = MatchSeparator::Chg(sep(split_pat));
        $format.push(pro);
    };
}

#[derive(Clone)]
enum Separator {
    Space(SplitPattern),
    Newline,
    Paragraph,
    Multispace,
    Period(SplitPattern),
    LiteralStr(LitStr),
    LiteralChar(LitChar),
}

macro_rules! parse_sep {
    ($format_context: ident, $input: ident, $sep: ident) => {
        let $sep;
        if $input.peek(Token![,]) {
            $sep = Format::last_sep_space(&$format_context);
            $input.parse::<Token![,]>()?;
        } else if $input.peek(Token![;]) {
            $sep = Separator::Newline;
            $input.parse::<Token![;]>()?;
        } else if $input.peek(Token![#]) {
            $sep = Separator::Paragraph;
            $input.parse::<Token![#]>()?;
        } else if $input.peek(Token![~]) {
            $sep = Separator::Multispace;
            $input.parse::<Token![~]>()?;
        } else if $input.peek(Token![.]) {
            $sep = Format::last_sep_period(&$format_context);
            $input.parse::<Token![.]>()?;
        } else if $input.peek(LitStr) {
            $sep = Separator::LiteralStr($input.parse::<LitStr>()?);
        } else if $input.peek(LitChar) {
            $sep = Separator::LiteralChar($input.parse::<LitChar>()?);
        } else {
            return Err($input.error("Expected separator (one of ,;#~. or string/char literal)!"));
        }
    };
}

#[allow(clippy::large_enum_variant)]
enum Match {
    Blank,
    Var(Box<Variable>),
    // repetition (inner, separator, collect)
    Rep(Vec<MatchSeparator>, Separator, bool),
}

mod mat {
    use crate::parse::Match;
    use crate::parse::{MatchSeparator, Variable};

    /// Return the `Var`s in `v`.
    fn vars(v: &Vec<MatchSeparator>) -> Vec<Variable> {
        let mut var = Vec::new();
        for ms in v {
            var.extend(match ms {
                MatchSeparator::Open(m) | MatchSeparator::Closed(m, _) => m.vars(),
                MatchSeparator::Chg(_) => vec![],
            })
        }
        var
    }

    impl Match {
        /// Return the variables in this `Match` as a vector.
        pub(crate) fn vars(&self) -> Vec<Variable> {
            match self {
                Match::Blank => vec![],
                Match::Var(var) => vec![*var.clone()],
                Match::Rep(match_separators, _, _) => vars(match_separators),
            }
        }
    }
}

impl ToTokens for Match {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Match::Blank => {}
            Match::Var(variable) => {
                let var = variable.ident.clone();
                tokens.extend(match &variable.conversion_type {
                    Some((ty, true)) => quote! {
                        #var = #RETURN_DATA.parse::<#ty>().expect("Parsing failed!");
                    },
                    Some((ty, false)) => quote! {
                        #var = #RETURN_DATA.parse::<#ty>();
                    },
                    None => quote! {
                        #var = #RETURN_DATA;
                    },
                });
            }
            Match::Rep(match_separators, separator, collect) => {
                let var = self.vars().first().cloned().map(|v| v.ident);
                let decl = var.as_ref().map(|id| quote! {let #id;});
                let assign = var.as_ref().map_or(quote! {let _}, |id| quote! {#id});

                // get iterator
                tokens.extend(match separator {
                    Separator::Space(split_pattern) => {
                        quote! {let #ITER = #RETURN_DATA.split(#split_pattern);}
                    }
                    Separator::Newline => quote! {let #ITER = #RETURN_DATA.lines();},
                    Separator::Paragraph => quote! {
                        let #ITER = simpar::ParagraphIterable::paragraphs(#RETURN_DATA);
                    },
                    Separator::Multispace => {
                        quote! {let #ITER = #RETURN_DATA.split(' ').filter(|s| !s.is_empty());}
                    }
                    Separator::Period(split_pattern) => {
                        quote! {let #ITER = #RETURN_DATA.split(#split_pattern);}
                    }
                    Separator::LiteralStr(lit_str) => {
                        quote! {let #ITER = #RETURN_DATA.split(#lit_str);}
                    }
                    Separator::LiteralChar(lit_char) => {
                        quote! {let #ITER = #RETURN_DATA.split(#lit_char);}
                    }
                });

                let col = collect.then_some(quote! {.collect::<Vec<_>>()});
                tokens.extend(quote! {
                    #assign = #ITER.map(|#INPUT| {
                        #decl
                        #(#match_separators)*
                        #var
                    })#col;
                });
            }
        }
    }
}

enum MatchSeparator {
    Open(Match),
    Closed(Match, Separator),
    // separator change
    Chg(Separator),
}

impl ToTokens for MatchSeparator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ext = match self {
            MatchSeparator::Open(mat) => quote! {
                let #RETURN_DATA = #INPUT;
                #mat
            },
            MatchSeparator::Closed(mat, separator) => {
                tokens.extend(quote! {
                    let __parse_macro_find_input = #INPUT;
                    let #RETURN_DATA;
                });

                let find_index = match separator {
                    Separator::Space(split_pattern) => quote! {
                        let j = #INPUT.find(#split_pattern).expect("Did not find separator!");
                        (#RETURN_DATA, #INPUT) = #INPUT.split_at(j);
                        #INPUT = #INPUT.strip_prefix(#split_pattern).unwrap();
                    },
                    Separator::Multispace => quote! {
                        (#RETURN_DATA, #INPUT) = simpar::split_multispace(#INPUT).expect("Expected space (' ')!");
                    },
                    Separator::Newline => quote! {
                        (#RETURN_DATA, #INPUT) = simpar::split_line(#INPUT).expect("Expected newline!");
                    },
                    Separator::Paragraph => quote! {
                        (#RETURN_DATA, #INPUT) = simpar::split_paragraph(#INPUT).expect("Expected paragraph!");
                    },
                    Separator::Period(split_pattern) => quote! {
                        let j = #INPUT.find(#split_pattern).expect("Did not find separator!");
                        (#RETURN_DATA, #INPUT) = #INPUT.split_at(j);
                        #INPUT = #INPUT.strip_prefix(#split_pattern).unwrap();
                    },
                    Separator::LiteralStr(lit_str) => quote! {
                        let j = #INPUT.find(#lit_str).expect("Did not find separator!");
                        (#RETURN_DATA, #INPUT) = #INPUT.split_at(j);
                        #INPUT = #INPUT.strip_prefix(#lit_str).unwrap();
                    },
                    Separator::LiteralChar(lit_char) => quote! {
                        let j = #INPUT.find(#lit_char).expect("Did not find separator!");
                        (#RETURN_DATA, #INPUT) = #INPUT.split_at(j);
                        #INPUT = #INPUT.strip_prefix(#lit_char).unwrap();
                    },
                };
                tokens.extend(find_index);

                quote! {
                    #mat
                }
            }
            MatchSeparator::Chg(_) => quote! {},
        };
        tokens.extend(ext);
    }
}

struct Format(Vec<MatchSeparator>);

mod format {
    use crate::parse::*;

    fn check_open(v: &[MatchSeparator]) -> bool {
        v[..(v.len() - 1)]
            .iter()
            .all(|ms| matches!(ms, MatchSeparator::Closed(_, _) | MatchSeparator::Chg(_)))
    }

    impl Format {
        pub(crate) fn check_open(&self) -> bool {
            check_open(&self.0)
        }

        pub(crate) fn check_rep(&self) -> bool {
            self.0.iter().all(|ms| match ms {
                MatchSeparator::Open(m) | MatchSeparator::Closed(m, _) => m.vars().len() <= 1,
                MatchSeparator::Chg(_) => true,
            })
        }

        pub(crate) fn vars(&self) -> Vec<Variable> {
            self.0
                .iter()
                .flat_map(|ms| match ms {
                    MatchSeparator::Open(m) | MatchSeparator::Closed(m, _) => m.vars(),
                    MatchSeparator::Chg(_) => vec![],
                })
                .collect()
        }

        pub(crate) fn last_sep_period(format: &[MatchSeparator]) -> Separator {
            format
                .iter()
                .rev()
                .find_map(|el| {
                    if let MatchSeparator::Chg(p) = el
                        && let Separator::Period(_) = p
                    {
                        Some(p)
                    } else {
                        None
                    }
                })
                .unwrap()
                .clone()
        }

        pub(crate) fn last_sep_space(format: &[MatchSeparator]) -> Separator {
            format
                .iter()
                .rev()
                .find_map(|el| {
                    if let MatchSeparator::Chg(p) = el
                        && let Separator::Space(_) = p
                    {
                        Some(p)
                    } else {
                        None
                    }
                })
                .unwrap()
                .clone()
        }
    }
}

impl syn::parse::Parse for Format {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // standard split patterns
        let mut format = vec![
            MatchSeparator::Chg(Separator::Period(SplitPattern::Char(LitChar::new(
                '.',
                Span::call_site(),
            )))),
            MatchSeparator::Chg(Separator::Space(SplitPattern::Char(LitChar::new(
                ' ',
                Span::call_site(),
            )))),
        ];

        while !input.is_empty() {
            let mat;
            if input.peek(Token![_]) {
                // blank match
                input.parse::<Token![_]>()?;
                mat = Match::Blank;
            } else if input.peek(Ident) || input.peek(Token![mut]) {
                // output ident
                let mu = input.parse::<Token![mut]>().ok();
                let id = input.parse::<Ident>()?;
                let ty = input
                    .peek(Token![:])
                    .then(|| {
                        input.parse::<Token![:]>().unwrap();
                        let ty = input.parse::<Type>()?;
                        if input.peek(Token![?]) {
                            input.parse::<Token![?]>().unwrap();
                            Ok((ty, false))
                        } else {
                            Ok((ty, true))
                        }
                    })
                    .map_or(Ok(None), |y: syn::Result<(Type, bool)>| y.map(Some))?;

                let var = Variable {
                    mutability: mu,
                    ident: id,
                    conversion_type: ty,
                };

                // make Match
                mat = Match::Var(Box::new(var));
            } else if input.peek(Paren) {
                let inner;
                parenthesized!(inner in input);

                let Format(inner_format) = inner.parse::<Format>()?;

                // get rep separator
                input.parse::<Token![*]>()?;
                parse_sep!(format, input, sep);

                mat = Match::Rep(inner_format, sep, false);
            } else if input.peek(Bracket) {
                let inner;
                bracketed!(inner in input);

                let Format(inner_format) = inner.parse::<Format>()?;

                // get rep separator
                input.parse::<Token![*]>()?;
                parse_sep!(format, input, sep);

                mat = Match::Rep(inner_format, sep, true);
            } else if input.peek(Brace) {
                parse_spat!(input, format);
                continue;
            } else {
                // allow for consecutive separators by treating this as a `Blank`
                mat = Match::Blank;
                // this will panic later in `parse_sep!` if there is an unexpected token
            }

            if input.is_empty() {
                format.push(MatchSeparator::Open(mat));
                break;
            }

            // parse split pattern between match and separator
            if input.peek(Brace) {
                parse_spat!(input, format);
            }

            // get Separator
            parse_sep!(format, input, sep);

            // make MatchSeparator and push
            format.push(MatchSeparator::Closed(mat, sep));
        }

        Ok(Self(format))
    }
}

enum Data {
    Expr(Expr),
}

impl ToTokens for Data {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Data::Expr(expr) => expr.to_tokens(tokens),
        }
    }
}

impl syn::parse::Parse for Data {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut expr_tokens = proc_macro2::TokenStream::new();

        // collect tokens into a separate buffer until we see an arrow
        while !input.is_empty() {
            if input.peek(Token![->]) {
                break;
            }

            // otherwise, move the token into an expression buffer
            let token: TokenTree = input.parse()?;
            expr_tokens.extend(std::iter::once(token));
        }

        let expr: Expr = syn::parse2(expr_tokens)?;
        Ok(Self::Expr(expr))
    }
}

struct Parser {
    data: Data,
    outputs: Vec<Variable>,
    format: Format,
}

impl Parser {
    fn check(self) -> CheckedParser {
        if !self.format.check_open() {
            panic!("Open match can only be used at the end of parser!");
        }

        if !self.format.check_rep() {
            panic!("Multivariable repetitions are unsupported.");
        }

        CheckedParser(self)
    }
}

/// Wrapper for `Parser` to ensure that
/// - `Open` is only used at the end of `Vec<..>`
/// - `Var` is used at most once in `Rep`.
struct CheckedParser(Parser);

impl CheckedParser {}

impl syn::parse::Parse for CheckedParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let data = input.parse::<Data>()?;

        input.parse::<Token![->]>()?;

        let format = input.parse::<Format>()?;
        let outputs = format.vars();

        Ok((Parser {
            data,
            outputs,
            format,
        })
        .check())
    }
}

pub fn parse_impl(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parser = parse_macro_input!(item as CheckedParser);

    let CheckedParser(Parser {
        data,
        outputs,
        format,
    }) = parser;

    let format = format.0;

    quote! {
        #(
            #outputs
        )*

        {
            // local variables
            let mut #INPUT = #data;

            #(
                #format
            )*
        }
    }
    .into()
}
