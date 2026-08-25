use std::slice::{Iter, IterMut};

use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Expr, Ident, LitChar, LitInt, LitStr, Token, Type, braced, bracketed, parenthesized,
    parse_macro_input,
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

struct Reference;

impl Reference {
    fn from_id(id: usize) -> Ident {
        Ident::new(
            &format!("__simpar_macro_internal_temp_{}", id),
            proc_macro2::Span::call_site(),
        )
    }
}

#[derive(Clone)]
enum ReferenceIdentifier {
    Id(Ident),
    Num(usize),
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
    fn parse_sep_chg(input: syn::parse::ParseStream, format: &mut Format) -> syn::Result<()> {
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
        format_context: &Format,
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
    // reference (ref, referenced)
    Ref(Variable, Ident),
    RefTemp(Option<(Type, bool)>, ReferenceIdentifier),
    // repetition (inner, separator, collect, root)
    Rep(Format, Separator, bool, bool),
}

mod mat {
    use proc_macro2::Span;

    use crate::parse::Match;
    use crate::parse::Variable;

    impl Match {
        /// Return the output variables in this `Match` as a vector.
        pub(crate) fn vars(&self) -> Vec<Variable> {
            match self {
                Match::Blank => vec![],
                Match::Var(var) => vec![var.clone()],
                Match::Rep(match_separators, _, _, _) => match_separators.vars(),
                Match::Ref(_, _) => vec![],
                Match::RefTemp(_, _) => vec![],
            }
        }

        pub(crate) fn var_to_ref(&mut self) {
            if let Match::Var(variable) = self {
                let ident = variable.ident.clone();
                let new_ident = syn::Ident::new(
                    &format!("__simpar_macro_internal_{}_ref", ident),
                    Span::call_site(),
                );
                let mut reference = variable.clone();
                reference.ident = new_ident;
                *self = Match::Ref(reference, ident);
            }
        }
    }
}

impl ToTokens for Match {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Match::Blank => {}
            Match::Var(variable) => {
                let var = &variable.ident;
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
                    let singles = match_separators.singles();
                    let reps = singles
                        .into_iter()
                        .map(|format| Match::Rep(format, separator.clone(), *collect, false));
                    tokens.extend(quote! {
                        #(#reps)*
                    });
                } else {
                    let var = match_separators.rep_return_ident();
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
                            #match_separators
                            #var
                        })#col;
                    });
                }
            }
            Match::Ref(reference, _) => {
                let var = &reference.ident;
                tokens.extend(match &reference.conversion_type {
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
            Match::RefTemp(_, _) => unreachable!("Error: reached `to_tokens` for `RefTemp`!"),
        }
    }
}

#[derive(Clone)]
enum MatchSeparator {
    Open(Match),
    Closed(Match, Separator),
    // separator change
    Chg(SeparatorPattern),
    // reference combinator (inputs, output, root)
    Cmb(Vec<Ident>, Ident, Ident),
    // reference combinator that returns an output variable
    CmbRoot(Vec<Ident>, Variable),
    // does nothing, similar to `Match::Blank` for `MatchSeparator`
    Dummy,
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
            MatchSeparator::Cmb(idents, var, _) => quote! {
                #var = (#(#idents),*);
            },
            MatchSeparator::CmbRoot(
                idents,
                Variable {
                    mutability: _,
                    ident,
                    conversion_type: _,
                },
            ) => quote! {
                #ident = (#(#idents),*);
            },
            MatchSeparator::Dummy => return,
        };
        tokens.extend(ext);
    }
}

#[derive(Clone)]
struct Format(Vec<MatchSeparator>);

impl Format {
    fn push(&mut self, item: MatchSeparator) {
        self.0.push(item);
    }
}

impl<'a> IntoIterator for &'a Format {
    type Item = &'a MatchSeparator;

    type IntoIter = Iter<'a, MatchSeparator>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Format {
    type Item = &'a mut MatchSeparator;

    type IntoIter = IterMut<'a, MatchSeparator>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

mod format {
    use crate::parse::*;
    use std::collections::HashSet;

    impl Format {
        // Set the `root` attribute of all repetitions to `false`.
        fn validate_not_root(&mut self) {
            for m in self.matches_mut() {
                match m {
                    Match::Blank => {}
                    Match::Var(_) => {}
                    Match::Rep(match_separators, _, _, root) => {
                        *root = false;
                        Self::validate_not_root(match_separators);
                    }
                    Match::Ref(_, _) => {}
                    Match::RefTemp(_, _) => {}
                }
            }
        }

        pub(crate) fn validate_root(&mut self) {
            for m in self.matches_mut() {
                match m {
                    Match::Blank => {}
                    Match::Var(_) => {}
                    Match::Rep(match_separators, _, _, root) => {
                        *root = true;
                        Self::validate_not_root(match_separators);
                    }
                    Match::Ref(_, _) => {}
                    Match::RefTemp(_, _) => {}
                }
            }
        }

        pub(crate) fn matches(&self) -> impl Iterator<Item = &Match> {
            self.0.iter().filter_map(|ms| match ms {
                MatchSeparator::Open(m) | MatchSeparator::Closed(m, _) => Some(m),
                _ => None,
            })
        }

        pub(crate) fn matches_mut(&mut self) -> impl Iterator<Item = &mut Match> {
            self.0.iter_mut().filter_map(|ms| match ms {
                MatchSeparator::Open(m) | MatchSeparator::Closed(m, _) => Some(m),
                _ => None,
            })
        }

        // Return all output variables.
        pub(crate) fn vars(&self) -> Vec<Variable> {
            self.0
                .iter()
                .flat_map(|ms| match ms {
                    MatchSeparator::Open(m) | MatchSeparator::Closed(m, _) => m.vars(),
                    MatchSeparator::Chg(_) => vec![],
                    MatchSeparator::Cmb(_, _, _) => vec![],
                    MatchSeparator::CmbRoot(_, variable) => vec![variable.clone()],
                    MatchSeparator::Dummy => vec![],
                })
                .collect()
        }

        fn referenced_vars(&self) -> Vec<Variable> {
            let vars = self.vars();
            let referenced: HashSet<&Ident> = HashSet::from_iter(self.referenced());

            vars.into_iter()
                .filter(|v| referenced.contains(&v.ident))
                .collect()
        }

        // Return the return variable inside a repetition or `None` if nothing should be returned.
        pub(crate) fn rep_return_ident(&self) -> Option<&Ident> {
            // check for combinators first
            for ms in self {
                match ms {
                    MatchSeparator::Cmb(_, ident, _) => return Some(ident),
                    MatchSeparator::CmbRoot(_, var) => return Some(&var.ident),
                    _ => {}
                }
            }

            // look for `Var`/`Ref` or inside of repetitions
            for m in self.matches() {
                match m {
                    Match::Blank => {}
                    Match::Var(variable) => return Some(&variable.ident),
                    Match::Ref(reference, _) => return Some(&reference.ident),
                    Match::Rep(format, _, _, _) => {
                        if let return_ident @ Some(_) = format.rep_return_ident() {
                            return return_ident;
                        }
                    }
                    Match::RefTemp(_, _) => {}
                };
            }
            None
        }

        // Returns all root identifiers in a repetition.
        fn roots(&self) -> Vec<&Ident> {
            let mut roots = HashSet::new();

            for ms in self {
                let m = match ms {
                    MatchSeparator::Dummy => continue,
                    MatchSeparator::Open(m) => m,
                    MatchSeparator::Closed(m, _) => m,
                    MatchSeparator::Cmb(_, _, root) => {
                        roots.insert(root);
                        continue;
                    }
                    MatchSeparator::CmbRoot(_, var) => {
                        roots.insert(&var.ident);
                        continue;
                    }
                    MatchSeparator::Chg(_) => continue,
                };

                match m {
                    Match::Blank => {}
                    Match::Var(variable) => {
                        roots.insert(&variable.ident);
                    }
                    Match::Ref(_, root) => {
                        roots.insert(root);
                    }
                    Match::Rep(format, _, _, _) => {
                        roots.extend(format.roots());
                    }
                    Match::RefTemp(_, _) => {}
                }
            }

            roots.into_iter().collect()
        }

        // Returns all referenced identifiers in a repetition.
        fn referenced(&self) -> Vec<&Ident> {
            let mut roots = HashSet::new();

            for m in self.matches() {
                match m {
                    Match::Blank => {}
                    Match::Var(_) => {}
                    Match::Ref(_, ident) => {
                        roots.insert(ident);
                    }
                    Match::Rep(format, _, _, _) => {
                        roots.extend(format.referenced());
                    }
                    Match::RefTemp(_, _) => {}
                }
            }

            roots.into_iter().collect()
        }

        pub(crate) fn last_sep_period(&self) -> SeparatorPattern {
            self.0
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

        pub(crate) fn last_sep_space(&self) -> SeparatorPattern {
            self.0
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

        fn clean_ident(&mut self, var: &Ident) {
            for ms in self {
                let m = match ms {
                    MatchSeparator::Open(m) => m,
                    MatchSeparator::Closed(m, _) => m,
                    MatchSeparator::Chg(_) => continue,
                    MatchSeparator::Cmb(_, _, root) => {
                        if root != var {
                            *ms = MatchSeparator::Dummy;
                        }
                        continue;
                    }
                    MatchSeparator::CmbRoot(_, variable) => {
                        if &variable.ident != var {
                            *ms = MatchSeparator::Dummy;
                        }
                        continue;
                    }
                    MatchSeparator::Dummy => continue,
                };
                match m {
                    Match::Blank => {}
                    Match::Var(variable) => {
                        if &variable.ident != var {
                            *m = Match::Blank;
                        }
                    }
                    Match::Ref(_, ident) => {
                        if ident != var {
                            *m = Match::Blank;
                        }
                    }
                    Match::Rep(format, _, _, _) => format.clean_ident(var),
                    Match::RefTemp(_, _) => {}
                }
            }
        }

        /// Returns copies of `format` for each `Variable` such that each copy has exactly one `Variable`
        /// or one exact copy if `format` has none.
        pub(crate) fn singles(&self) -> Vec<Format> {
            let vars = self.roots();

            // if `format` has zero variables return a copy of `self`
            if vars.is_empty() {
                return vec![self.clone()];
            }

            let mut singles = Vec::new();
            for root in vars {
                let mut copy = self.clone();

                copy.clean_ident(root);
                singles.push(copy);
            }
            singles
        }

        /// Returns a path to the (unique) combinator root or `None` if there are no references.
        /// Paths are vectors of indces, where the last element corresponds to the direct child
        /// of `self`.
        fn get_root(&self, var: &Ident) -> Option<Vec<usize>> {
            let mut references = 0;
            let mut root = None;

            for (i, ms) in self.into_iter().enumerate() {
                let m = match ms {
                    MatchSeparator::Open(m) | MatchSeparator::Closed(m, _) => m,
                    _ => continue,
                };
                match m {
                    Match::Blank => {}
                    Match::Var(variable) if &variable.ident == var => {
                        references += 1;
                        root = Some(vec![]);
                    }
                    Match::Ref(_, ident) if ident == var => {
                        references += 1;
                        root = Some(vec![]);
                    }
                    Match::Rep(format, _, _, _) => {
                        if let Some(mut tail) = format.get_root(var) {
                            references += 1;
                            tail.push(i);
                            root = Some(tail);
                        }
                    }
                    _ => {}
                }
            }

            if references > 1 {
                // self is root
                Some(vec![])
            } else if references == 1 {
                // either exactly one child that is a reference or
                // exactly one child that leads to root
                root
            } else {
                // no reference in self
                None
            }
        }

        /// Inserts all combinators for `var`.
        /// Assumes that `self` is the root for combining `var` and inserts
        /// `MatchSeparator::CmbRoot` into `self` and `MatchSeparator::Cmb` into children.
        fn insert_cmb_root(&mut self, var: Variable, cmb_i: &mut usize) {
            let var_ident = &var.ident;
            let mut inputs = vec![];

            for m in self.matches_mut() {
                match m {
                    Match::Var(variable) if &variable.ident == var_ident => {
                        // change to Ref
                        m.var_to_ref();
                        // add new ident to inputs
                        match m {
                            Match::Ref(variable, _) => inputs.push(variable.ident.clone()),
                            _ => unreachable!(),
                        }
                    }
                    Match::Ref(variable, ident) if ident == var_ident => {
                        inputs.push(variable.ident.clone());
                    }
                    Match::Rep(format, _, _, _) => {
                        if let Some(rep_output) = format.insert_cmb_not_root(var_ident, cmb_i) {
                            inputs.push(rep_output);
                        }
                    }
                    _ => {}
                }
            }

            // sanity check: there should be at least two inputs
            if inputs.len() < 2 {
                panic!("unexpected combinator");
            }

            self.push(MatchSeparator::CmbRoot(inputs, var));
        }

        /// Inserts all combinators for `var` and return the last output or `None` if there is
        /// no reference.
        /// Assumes that `self` is not the root for combining `var` and only inserts
        /// `MatchSeparator::Cmb`.
        fn insert_cmb_not_root(&mut self, var: &Ident, cmb_i: &mut usize) -> Option<Ident> {
            // combinator inputs
            let mut inputs: Vec<Ident> = vec![];

            for m in self.matches_mut() {
                match m {
                    Match::Var(variable) if &variable.ident == var => {
                        // change to Ref
                        m.var_to_ref();
                        // add new ident to inputs
                        match m {
                            Match::Ref(variable, _) => inputs.push(variable.ident.clone()),
                            _ => unreachable!(),
                        }
                    }
                    Match::Ref(variable, ident) if ident == var => {
                        inputs.push(variable.ident.clone());
                    }
                    Match::Rep(format, _, _, _) => {
                        if let Some(rep_output) = format.insert_cmb_not_root(var, cmb_i) {
                            inputs.push(rep_output);
                        }
                    }
                    _ => {}
                }
            }

            if inputs.len() > 1 {
                // need combinator
                let output = Ident::new(
                    &format!("__simpar_macro_internal_cmb_{}", cmb_i),
                    proc_macro2::Span::call_site(),
                );
                *cmb_i += 1;
                self.push(MatchSeparator::Cmb(inputs, output.clone(), var.clone()));
                Some(output)
            } else if inputs.len() == 1 {
                inputs.pop()
            } else {
                None
            }
        }

        pub(crate) fn insert_cmb(&mut self) {
            let mut cmb_i = 0;
            for var in self.referenced_vars() {
                let var_ident = &var.ident;

                if let Some(mut path) = self.get_root(var_ident) {
                    // -> combinator needed

                    // move to root
                    let mut head = &mut *self;
                    while let Some(i) = path.pop() {
                        let child = &mut head.0[i];
                        let m = match child {
                            MatchSeparator::Open(m) | MatchSeparator::Closed(m, _) => m,
                            _ => panic!("unsound index: expected match"),
                        };
                        head = match m {
                            Match::Rep(format, _, _, _) => format,
                            _ => panic!("unsound index: expected repetition"),
                        }
                    }

                    head.insert_cmb_root(var, &mut cmb_i);
                }
            }
        }

        pub(crate) fn validate_refs(&mut self, vars: &[Variable], reference_id: &mut usize) {
            for m in self.matches_mut() {
                match m {
                    Match::Blank => {}
                    Match::Var(_) => {}
                    Match::Ref(_, _) => {}
                    Match::RefTemp(ty, reference_identifier) => {
                        let ty = ty.clone();
                        let ident = match reference_identifier {
                            ReferenceIdentifier::Id(ident) => ident.clone(),
                            ReferenceIdentifier::Num(i) => {
                                vars.get(*i).expect("Index out of bounds!").ident.clone()
                            }
                        };
                        let _ = std::mem::replace(
                            m,
                            Match::Ref(
                                Variable {
                                    mutability: None,
                                    ident: Reference::from_id(*reference_id),
                                    conversion_type: ty,
                                },
                                ident,
                            ),
                        );
                        *reference_id += 1;
                    }
                    Match::Rep(format, _, _, _) => {
                        format.validate_refs(vars, reference_id);
                    }
                }
            }
        }
    }
}

impl ToTokens for Format {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // temporary variables
        let decl = self
            .into_iter()
            .flat_map(|ms| match ms {
                MatchSeparator::Cmb(idents, _, _) => idents.iter(),
                MatchSeparator::CmbRoot(idents, _) => idents.iter(),
                _ => [].iter(),
            })
            .map(|idents| quote! {let #idents;});

        let inner = &self.0;
        tokens.extend(quote! {
            #(#decl)*
            #(#inner)*
        });
    }
}

impl syn::parse::Parse for Format {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // default split patterns
        let mut format = Format(vec![
            MatchSeparator::Chg(SeparatorPattern::Period(SplitPattern::DefaultPeriod)),
            MatchSeparator::Chg(SeparatorPattern::Space(SplitPattern::DefaultSpace)),
        ]);

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

                let inner_format = inner.parse::<Format>()?;

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

                let inner_format = inner.parse::<Format>()?;

                // get rep separator
                let sep = Separator::parse_separaror(&format, input)?;
                input.parse::<Token![*]>()?;

                mat = Match::Rep(inner_format, sep, true, false);
            } else if input.peek(Brace) {
                SplitPattern::parse_sep_chg(input, &mut format)?;
                continue;
            } else if input.peek(Token![$]) {
                input.parse::<Token![$]>()?;

                let ref_id = if input.peek(LitInt) {
                    let num = input.parse::<LitInt>()?.base10_parse::<usize>()?;
                    ReferenceIdentifier::Num(num)
                } else if input.peek(Ident) {
                    let var = input.parse::<Ident>()?;
                    ReferenceIdentifier::Id(var)
                } else {
                    return Err(input.error("expected identifier or integer literal"));
                };

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

                mat = Match::RefTemp(ty, ref_id);
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

        Ok(format)
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

        let vars_ordered = self.format.vars();
        self.format.validate_refs(&vars_ordered, &mut 0);

        self.format.insert_cmb();

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

    quote! {
        #(
            #outputs
        )*

        {
            // local variables
            let mut #INPUT = #data;

            #format
        }
    }
    .into()
}
