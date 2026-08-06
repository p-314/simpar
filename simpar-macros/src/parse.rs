use proc_macro2::{TokenStream, TokenTree};
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
const CONDENSE_NON_EMPTY: IdentHelper = new_ident!("condense_non_empty");

/// Variable type for identifiers in return positions
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
        } = self;
        tokens.extend(quote! {let #mu #id;});
    }
}

/// Type for the changeable value of programmable separators.
#[derive(Clone)]
enum SplitPattern {
    DefaultSpace,
    DefaultPeriod,
    Pattern(Expr),
}

impl ToTokens for SplitPattern {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            SplitPattern::Pattern(expr) => expr.to_tokens(tokens),
            SplitPattern::DefaultSpace => quote! {' '}.to_tokens(tokens),
            SplitPattern::DefaultPeriod => quote! {'.'}.to_tokens(tokens),
        }
    }
}

impl SplitPattern {
    /// Parse `{<sep> = <pat>}` into `MatchSeparator::Chg`
    fn parse_sep_chg(
        input: syn::parse::ParseStream,
        format: &mut Vec<MatchSeparator>,
    ) -> syn::Result<()> {
        let inner;
        braced!(inner in input);

        let sep = if inner.peek(Token![.]) {
            inner.parse::<Token![.]>()?;
            SeparatorPattern::Period
        } else if inner.peek(Token![,]) {
            inner.parse::<Token![,]>()?;
            SeparatorPattern::Space
        } else {
            return Err(input.error("Expected programmable separator (, or .)!"));
        };

        inner.parse::<Token![=]>()?;

        let split_pat = SplitPattern::Pattern(inner.parse::<Expr>()?);

        let pro = MatchSeparator::Chg(sep(split_pat));
        format.push(pro);
        Ok(())
    }
}

#[derive(Clone)]
struct Separator {
    pat: SeparatorPattern,
    condensed: bool,
}

#[derive(Clone)]
enum SeparatorPattern {
    Space(SplitPattern),
    Newline,
    Paragraph,
    Period(SplitPattern),
    LiteralStr(LitStr),
    LiteralChar(LitChar),
    ByteOffset(Expr),
}

impl Separator {
    fn parse_separaror(
        format_context: &[MatchSeparator],
        input: syn::parse::ParseStream,
    ) -> syn::Result<Self> {
        let pat;
        if input.peek(Token![,]) {
            pat = Format::last_sep_space(format_context);
            input.parse::<Token![,]>()?;
        } else if input.peek(Token![;]) {
            pat = SeparatorPattern::Newline;
            input.parse::<Token![;]>()?;
        } else if input.peek(Token![#]) {
            pat = SeparatorPattern::Paragraph;
            input.parse::<Token![#]>()?;
        } else if input.peek(Token![.]) {
            pat = Format::last_sep_period(format_context);
            input.parse::<Token![.]>()?;
        } else if input.peek(LitStr) {
            pat = SeparatorPattern::LiteralStr(input.parse::<LitStr>()?);
        } else if input.peek(LitChar) {
            pat = SeparatorPattern::LiteralChar(input.parse::<LitChar>()?);
        } else if input.peek(Bracket) {
            let inner;
            bracketed!(inner in input);
            inner.parse::<Token![+]>()?;
            pat = SeparatorPattern::ByteOffset(inner.parse::<Expr>()?);
        } else {
            return Err(
                input.error("Expected separator (one of ,;#~.[+i] or string/char literal)!")
            );
        }

        let condensed = if input.peek(Token![~]) {
            input.parse::<Token![~]>()?;
            true
        } else {
            false
        };

        Ok(Separator { pat, condensed })
    }
}

#[derive(Clone)]
enum Match {
    Blank,
    Var(Variable),
    // repetition (inner, separator, collect, root)
    Rep(Vec<MatchSeparator>, Separator, bool, bool),
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
                Match::Var(var) => vec![var.clone()],
                Match::Rep(match_separators, _, _, __simpar_macro_internal_) => {
                    vars(match_separators)
                }
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
            Match::Rep(match_separators, separator, collect, root) => {
                if *root {
                    let singles = Format::into_single_vars(match_separators);
                    let reps = singles
                        .into_iter()
                        .map(|format| Match::Rep(format, separator.clone(), *collect, false));
                    tokens.extend(quote! {
                        #(#reps)*
                    });
                } else {
                    let var = self.vars().first().cloned().map(|v| v.ident);
                    let decl = var.as_ref().map(|id| quote! {let #id;});
                    let assign = var.as_ref().map_or(quote! {let _}, |id| quote! {#id});

                    let condense = separator
                        .condensed
                        .then_some(quote! {.filter(|s| !s.is_empty())});

                    // get iterator
                    tokens.extend(match &separator.pat {
                        SeparatorPattern::Space(split_pattern) => quote! {
                            let #ITER = #RETURN_DATA.split(#split_pattern)#condense;
                        },
                        SeparatorPattern::Newline => quote! {let #ITER = #RETURN_DATA.lines()#condense;},
                        SeparatorPattern::Paragraph => quote! {
                            let #ITER = simpar::ParagraphIterable::paragraphs(#RETURN_DATA)#condense;
                        },
                        SeparatorPattern::Period(split_pattern) => quote! {
                            let #ITER = #RETURN_DATA.split(#split_pattern)#condense;
                        },
                        SeparatorPattern::LiteralStr(lit_str) => quote! {
                            let #ITER = #RETURN_DATA.split(#lit_str)#condense;
                        },
                        SeparatorPattern::LiteralChar(lit_char) => quote! {
                            let #ITER = #RETURN_DATA.split(#lit_char)#condense;
                        },
                        SeparatorPattern::ByteOffset(lit_int) => quote! {
                            let #ITER = #RETURN_DATA.as_bytes().chunks(#lit_int).map(|slice| str::from_utf8(slice).expect("Index outside char boundary!"));
                        },
                    });

                    let col = collect.then_some(quote! {.collect::<Vec<_>>()});
                    tokens.extend(quote! {
                        #assign = #ITER.map(|mut #INPUT| {
                            #decl
                            #(#match_separators)*
                            #var
                        })#col;
                    });
                }
            }
        }
    }
}

#[derive(Clone)]
enum MatchSeparator {
    Open(Match),
    Closed(Match, Separator),
    // separator change
    Chg(SeparatorPattern),
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
                    let #RETURN_DATA;
                });

                let find_index = match &separator.pat {
                    SeparatorPattern::Space(split_pattern) => quote! {
                        let j = #INPUT.find(#split_pattern).expect("Did not find separator!");
                        (#RETURN_DATA, #INPUT) = #INPUT.split_at(j);
                        #INPUT = #INPUT.strip_prefix(#split_pattern).unwrap();
                    },
                    SeparatorPattern::Newline => quote! {
                        (#RETURN_DATA, #INPUT) = simpar::split_line(#INPUT).expect("Expected newline!");
                    },
                    SeparatorPattern::Paragraph => quote! {
                        (#RETURN_DATA, #INPUT) = simpar::split_paragraph(#INPUT).expect("Expected paragraph!");
                    },
                    SeparatorPattern::Period(split_pattern) => quote! {
                        let j = #INPUT.find(#split_pattern).expect("Did not find separator!");
                        (#RETURN_DATA, #INPUT) = #INPUT.split_at(j);
                        #INPUT = #INPUT.strip_prefix(#split_pattern).unwrap();
                    },
                    SeparatorPattern::LiteralStr(lit_str) => quote! {
                        let j = #INPUT.find(#lit_str).expect("Did not find separator!");
                        (#RETURN_DATA, #INPUT) = #INPUT.split_at(j);
                        #INPUT = #INPUT.strip_prefix(#lit_str).unwrap();
                    },
                    SeparatorPattern::LiteralChar(lit_char) => quote! {
                        let j = #INPUT.find(#lit_char).expect("Did not find separator!");
                        (#RETURN_DATA, #INPUT) = #INPUT.split_at(j);
                        #INPUT = #INPUT.strip_prefix(#lit_char).unwrap();
                    },
                    SeparatorPattern::ByteOffset(offset) => quote! {
                        (#RETURN_DATA, #INPUT) = #INPUT.split_at(#offset);
                    },
                };
                tokens.extend(find_index);

                if separator.condensed {
                    let condense = match &separator.pat {
                        SeparatorPattern::Space(split_pattern) => {
                            quote! {#INPUT = #INPUT.trim_start_matches(#split_pattern);}
                        }
                        SeparatorPattern::Newline => quote! {
                            if let Some(#CONDENSE_NON_EMPTY) = #INPUT.lines().filter(|line| !line.is_empty()).next() {
                                #INPUT = simpar::subslice_extend_right(#CONDENSE_NON_EMPTY, #INPUT);
                            } else {
                                #INPUT = &#INPUT[#INPUT.len()..];
                            }
                        },
                        SeparatorPattern::Paragraph => quote! {
                            if let Some(#CONDENSE_NON_EMPTY) = simpar::ParagraphIterable::paragraphs(#INPUT).filter(|par| !par.is_empty()).next() {
                                #INPUT = simpar::subslice_extend_right(#CONDENSE_NON_EMPTY, #INPUT);
                            } else {
                                #INPUT = &#INPUT[#INPUT.len()..];
                            }
                        },
                        SeparatorPattern::Period(split_pattern) => {
                            quote! {#INPUT = #INPUT.trim_start_matches(#split_pattern);}
                        }
                        SeparatorPattern::LiteralStr(lit_str) => {
                            quote! {#INPUT = #INPUT.trim_start_matches(#lit_str);}
                        }
                        SeparatorPattern::LiteralChar(lit_char) => {
                            quote! {#INPUT = #INPUT.trim_start_matches(#lit_char);}
                        }
                        SeparatorPattern::ByteOffset(_) => quote! {},
                    };

                    tokens.extend(condense);
                }

                quote! {
                    #mat
                }
            }
            MatchSeparator::Chg(_) => return,
        };
        tokens.extend(ext);
    }
}

struct Format(Vec<MatchSeparator>);

mod format {
    use crate::parse::*;

    impl Format {
        fn validate_not_root(format: &mut [MatchSeparator]) {
            for ms in format {
                let m = match ms {
                    MatchSeparator::Open(m) => m,
                    MatchSeparator::Closed(m, _) => m,
                    MatchSeparator::Chg(_) => continue,
                };
                match m {
                    Match::Blank => {}
                    Match::Var(_) => {}
                    Match::Rep(match_separators, _, _, root) => {
                        *root = false;
                        Self::validate_not_root(match_separators);
                    }
                }
            }
        }

        pub(crate) fn validate_root(&mut self) {
            for ms in &mut self.0 {
                let m = match ms {
                    MatchSeparator::Open(m) => m,
                    MatchSeparator::Closed(m, _) => m,
                    MatchSeparator::Chg(_) => continue,
                };
                match m {
                    Match::Blank => {}
                    Match::Var(_) => {}
                    Match::Rep(match_separators, _, _, root) => {
                        *root = true;
                        Self::validate_not_root(match_separators);
                    }
                }
            }
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

        pub(crate) fn last_sep_period(format: &[MatchSeparator]) -> SeparatorPattern {
            format
                .iter()
                .rev()
                .find_map(|el| {
                    if let MatchSeparator::Chg(p @ SeparatorPattern::Period(_)) = el {
                        Some(p)
                    } else {
                        None
                    }
                })
                .unwrap()
                .clone()
        }

        pub(crate) fn last_sep_space(format: &[MatchSeparator]) -> SeparatorPattern {
            format
                .iter()
                .rev()
                .find_map(|el| {
                    if let MatchSeparator::Chg(p @ SeparatorPattern::Space(_)) = el {
                        Some(p)
                    } else {
                        None
                    }
                })
                .unwrap()
                .clone()
        }

        /// Extracts all `Match::Var` from `format` and returns them together with relative indices
        /// in **reverse** order.
        fn into_blanks(format: &mut [MatchSeparator]) -> (Vec<Match>, Vec<Vec<usize>>) {
            let mut vars = Vec::new();
            let mut indices = Vec::new();
            for (i, ms) in format.iter_mut().enumerate() {
                let mat = match ms {
                    MatchSeparator::Open(m) => m,
                    MatchSeparator::Closed(m, _) => m,
                    _ => continue,
                };
                match mat {
                    Match::Blank => {}
                    Match::Var(_) => {
                        vars.push(std::mem::replace(mat, Match::Blank));
                        indices.push(vec![i])
                    }
                    Match::Rep(match_separators, _, _, _) => {
                        let (u, j) = Format::into_blanks(match_separators);
                        vars.extend(u);
                        indices.extend(j.into_iter().map(|mut k| {
                            k.push(i);
                            k
                        }));
                    }
                }
            }
            (vars, indices)
        }

        pub(crate) fn into_single_vars(format: &[MatchSeparator]) -> Vec<Vec<MatchSeparator>> {
            let mut blank = format.to_owned();
            let (vars, indices) = Self::into_blanks(&mut blank);

            // if `format` has zero variables return `blank` (equal to `format`)
            if vars.is_empty() {
                return vec![blank];
            }

            let mut singles = Vec::new();
            'singles: for (v, mut ind) in vars.into_iter().zip(indices) {
                let mut copy = blank.clone();

                let mut vec_node = &mut copy;
                while let Some(i) = ind.pop() {
                    let node = &mut vec_node[i];
                    let node_match = match node {
                        MatchSeparator::Open(m) => m,
                        MatchSeparator::Closed(m, _) => m,
                        MatchSeparator::Chg(_) => unreachable!(),
                    };
                    match node_match {
                        Match::Rep(match_separators, _, _, _) => vec_node = match_separators,
                        _ => {
                            *node_match = v;
                            singles.push(copy);
                            continue 'singles;
                        }
                    };
                }
                unreachable!()
            }

            singles
        }
    }
}

impl syn::parse::Parse for Format {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // default split patterns
        let mut format = vec![
            MatchSeparator::Chg(SeparatorPattern::Period(SplitPattern::DefaultPeriod)),
            MatchSeparator::Chg(SeparatorPattern::Space(SplitPattern::DefaultSpace)),
        ];

        while !input.is_empty() {
            let mat;
            if input.peek(Token![_]) {
                // blank match
                input.parse::<Token![_]>()?;
                mat = Match::Blank;
            } else if input.peek(Ident) || input.peek(Token![mut]) {
                // ident in return position
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
                mat = Match::Var(var);
            } else if input.peek(Paren) {
                let inner;
                parenthesized!(inner in input);

                let Format(inner_format) = inner.parse::<Format>()?;

                // get rep separator
                let sep = Separator::parse_separaror(&format, input)?;
                input.parse::<Token![*]>()?;

                mat = Match::Rep(inner_format, sep, false, false);
            } else if input.peek(Bracket) {
                let inner;
                bracketed!(inner in input);

                // handle [+i] seperator
                if inner.peek(Token![+]) {
                    inner.parse::<Token![+]>()?;

                    let pat = SeparatorPattern::ByteOffset(inner.parse::<Expr>()?);
                    let condensed = input.peek(Token![~]);
                    if condensed {
                        input.parse::<Token![~]>()?;
                    }

                    // there must be a separator in front to be here => use `Closed` with `Blank`
                    format.push(MatchSeparator::Closed(
                        Match::Blank,
                        Separator { pat, condensed },
                    ));
                    continue;
                }

                let Format(inner_format) = inner.parse::<Format>()?;

                // get rep separator
                let sep = Separator::parse_separaror(&format, input)?;
                input.parse::<Token![*]>()?;

                mat = Match::Rep(inner_format, sep, true, false);
            } else if input.peek(Brace) {
                SplitPattern::parse_sep_chg(input, &mut format)?;
                continue;
            } else {
                // allow for consecutive separators by treating this as a `Blank`
                mat = Match::Blank;
                // this will panic later in `parse_separator` if there is an unexpected token
            }

            if input.is_empty() {
                format.push(MatchSeparator::Open(mat));
                break;
            }

            // parse separator change between match and separator
            if input.peek(Brace) {
                SplitPattern::parse_sep_chg(input, &mut format)?;
            }

            // get Separator
            let sep = Separator::parse_separaror(&format, input)?;

            // make MatchSeparator and push
            format.push(MatchSeparator::Closed(mat, sep));
        }

        Ok(Self(format))
    }
}

// Input data type.
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
    fn check(mut self) -> CheckedParser {
        self.format.validate_root();

        CheckedParser(self)
    }
}

/// Wrapper for `Parser` to ensure that
/// - `root` is set to correctly in all repetitions
struct CheckedParser(Parser);

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
