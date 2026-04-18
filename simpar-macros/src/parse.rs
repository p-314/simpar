use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Ident, LitStr, Token, Type, parenthesized, parse_macro_input, token::Paren};

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
const COMMA_SEPERATOR: IdentHelper = new_ident!("comma_seperator");
const RETURN_DATA: IdentHelper = new_ident!("return_data");
const ITER: IdentHelper = new_ident!("iter");

#[derive(Clone)]
struct Variable {
    mu: Option<Token![mut]>,
    id: Ident,
    ty: Option<Type>,
}

impl ToTokens for Variable {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { mu, id, ty: _ } = self.clone();
        tokens.extend(quote! {let #mu #id;});
    }
}

enum Seperator {
    Space,
    Newline,
    Block,
    Multispace,
}

macro_rules! parse_sep {
    ($input: ident, $sep: ident) => {
        let $sep;
        if $input.peek(Token![,]) {
            $sep = Seperator::Space;
            $input.parse::<Token![,]>()?;
        } else if $input.peek(Token![;]) {
            $sep = Seperator::Newline;
            $input.parse::<Token![;]>()?;
        } else if $input.peek(Token![#]) {
            $sep = Seperator::Block;
            $input.parse::<Token![#]>()?;
        } else if $input.peek(Token![~]) {
            $sep = Seperator::Multispace;
            $input.parse::<Token![~]>()?;
        } else {
            return Err($input.error("Expected seperator (one of ,;#~)!"))
        }
    };
}

#[allow(clippy::large_enum_variant)]
enum Match {
    Blank,
    Var(Box<Variable>),
    Rep(Vec<MatchSeperator>, Seperator),
}

mod mat {
    use crate::parse::Match;
    use crate::parse::{MatchSeperator, Variable};

    /// Return the `Var`s in `v`.
    fn vars(v: &Vec<MatchSeperator>) -> Vec<Variable> {
        let mut var = Vec::new();
        for ms in v {
            var.extend(match ms {
                MatchSeperator::Open(m) | MatchSeperator::Closed(m, _) => m.vars(),
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
                Match::Rep(match_seperators, _) => vars(match_seperators),
            }
        }
    }
}

impl ToTokens for Match {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Match::Blank => {}
            Match::Var(variable) => {
                let var = variable.id.clone();
                tokens.extend(match &variable.ty {
                    Some(ty) => quote! {
                        #var = #RETURN_DATA.parse::<#ty>().expect("Parsing failed!");
                    },
                    None => quote! {
                        #var = #RETURN_DATA;
                    },
                });
            }
            Match::Rep(match_seperators, seperator) => {
                let var = self.vars().first().cloned().map(|v| v.id);
                let decl = var.as_ref().map(|id| quote! {let #id;});
                let assign = var.as_ref().map_or(quote! {let _}, |id| quote! {#id});

                // get iterator
                tokens.extend(match seperator {
                    Seperator::Space => quote! {let #ITER = #RETURN_DATA.split(#COMMA_SEPERATOR);},
                    Seperator::Newline => quote! {let #ITER = #RETURN_DATA.lines();},
                    Seperator::Block => quote! {
                        let #ITER = simpar::BlockIterable::blocks(#RETURN_DATA);
                    },
                    Seperator::Multispace => {
                        quote! {let #ITER = #RETURN_DATA.split(' ').filter(|s| !s.is_empty());}
                    }
                });

                tokens.extend(quote! {
                    #assign = #ITER.map(|#INPUT| {
                        #decl
                        #(#match_seperators)*
                        #var
                    });
                });
            }
        }
    }
}

enum MatchSeperator {
    Open(Match),
    Closed(Match, Seperator),
}

impl ToTokens for MatchSeperator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ext = match self {
            MatchSeperator::Open(mat) => quote! {
                let #RETURN_DATA = #INPUT;
                #mat
            },
            MatchSeperator::Closed(mat, seperator) => {
                tokens.extend(quote! {
                    let __parse_macro_find_input = #INPUT;
                    let #RETURN_DATA;
                });

                let find_index = match seperator {
                    Seperator::Space => quote! {
                        let j = #INPUT.find(#COMMA_SEPERATOR).expect("Expected space (' ')!");
                        (#RETURN_DATA, #INPUT) = #INPUT.split_at(j);
                        #INPUT = &#INPUT[#COMMA_SEPERATOR.len()..];
                    },
                    Seperator::Multispace => quote! {
                        (#RETURN_DATA, #INPUT) = simpar::split_multispace(#INPUT).expect("Expected space (' ')!");
                    },
                    Seperator::Newline => quote! {
                        (#RETURN_DATA, #INPUT) = simpar::split_line(#INPUT).expect("Expected newline!");
                    },
                    Seperator::Block => quote! {
                        (#RETURN_DATA, #INPUT) = simpar::split_block(#INPUT).expect("Expected block!");
                    },
                };
                tokens.extend(find_index);

                quote! {
                    #mat
                }
            }
        };
        tokens.extend(ext);
    }
}

struct Format(Vec<MatchSeperator>);

mod format {
    use crate::parse::*;

    fn check_open(v: &[MatchSeperator]) -> bool {
        v[..(v.len() - 1)]
            .iter()
            .all(|ms| matches!(ms, MatchSeperator::Closed(_, _)))
    }

    impl Format {
        pub(crate) fn check_open(&self) -> bool {
            check_open(&self.0)
        }

        pub(crate) fn check_rep(&self) -> bool {
            self.0.iter().all(|ms| match ms {
                MatchSeperator::Open(m) | MatchSeperator::Closed(m, _) => m.vars().len() <= 1,
            })
        }

        pub(crate) fn vars(&self) -> Vec<Variable> {
            self.0
                .iter()
                .flat_map(|ms| match ms {
                    MatchSeperator::Open(m) | MatchSeperator::Closed(m, _) => m.vars(),
                })
                .collect()
        }
    }
}

impl syn::parse::Parse for Format {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut format = Vec::new();

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
                        input.parse::<Token![:]>()?;
                        input.parse::<Type>()
                    })
                    .map_or(Ok(None), |y| y.map(Some))?;

                let var = Variable { mu, id, ty };

                // make Match
                mat = Match::Var(Box::new(var));
            } else if input.peek(Paren) {
                let inner;
                parenthesized!(inner in input);

                let Format(format) = inner.parse::<Format>()?;

                // get rep seperator
                input.parse::<Token![*]>()?;
                parse_sep!(input, sep);

                mat = Match::Rep(format, sep);
            } else {
                return Err(input.error("Unexpected token"));
            }

            if input.is_empty() {
                format.push(MatchSeperator::Open(mat));
                break;
            }

            // get Seperator
            parse_sep!(input, sep);

            // make MatchSeperator and push
            format.push(MatchSeperator::Closed(mat, sep));
        }

        Ok(Self(format))
    }
}

enum Data {
    Str(LitStr),
    Id(Ident),
}

impl ToTokens for Data {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Data::Str(lit_str) => lit_str.to_tokens(tokens),
            Data::Id(ident) => ident.to_tokens(tokens),
        }
    }
}

impl syn::parse::Parse for Data {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            return Ok(Self::Str(input.parse::<LitStr>().unwrap()));
        }
        if input.peek(Ident) {
            return Ok(Self::Id(input.parse::<Ident>().unwrap()));
        }
        Err(input.error("Expected identifier of string literal."))
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
            todo!("cant handle multivariable repetitions");
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

        // FIX: make this work scoped
        let mut #COMMA_SEPERATOR = " ";

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
