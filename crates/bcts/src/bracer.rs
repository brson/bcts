use rmx::prelude::*;

use rmx::core::ops::Range;
use rmx::core::iter::Peekable;
use rmx::std::io::Write;

use crate::chunk::Chunk;
use crate::lexer::{ChunkLex, Token, TokenKind, Sigil};

#[salsa::tracked]
pub struct Bracer<'db> {
    pub chunk: ChunkLex<'db>,
    #[returns(ref)]
    pub branches: Vec<Branch>,
    #[returns(ref)]
    pub inserted_closes: Vec<(usize, Sigil)>,
    #[returns(ref)]
    pub removed_closes: Vec<(usize, Sigil)>,
    #[returns(ref)]
    pub errors: Vec<(Range<usize>, Sigil)>,
}

#[derive(Clone, Debug, Hash, salsa::Update)]
pub struct Branch {
    real_token_range: Range<usize>,
    branches: usize,
    inserted_closes: usize,
    removed_closes: usize,
    errors: usize,
    open_sigil: Sigil,
    close_sigil: Sigil,
}

impl<'db> Bracer<'db> {
    pub fn iter(
        &self,
        db: &'db dyn crate::Db,
    ) -> BracerIter<'db> {
        BracerIter {
            db,
            tree: *self,
            real_token_range: 0..self.chunk(db).tokens(db).len(),
            branches: 0..self.branches(db).len(),
            inserted_closes: 0..self.inserted_closes(db).len(),
            removed_closes: 0..self.removed_closes(db).len(),
            next_token_index: 0,
            next_branch_index: 0,
            next_inserted_close_index: 0,
            next_removed_close_index: 0,
        }
    }
}

#[derive(Clone)]
pub struct BracerIter<'db> {
    pub db: &'db dyn crate::Db,
    tree: Bracer<'db>,
    real_token_range: Range<usize>,
    branches: Range<usize>,
    inserted_closes: Range<usize>,
    removed_closes: Range<usize>,
    next_token_index: usize,
    next_branch_index: usize,
    next_inserted_close_index: usize,
    next_removed_close_index: usize,
}

impl<'db> Iterator for BracerIter<'db> {
    type Item = TreeToken<'db>;

    fn next(&mut self) -> Option<TreeToken<'db>> {
        let res = self.next2();
        debug!("next: {:?}", match res.as_ref() {
            None => "none",
            Some(TreeToken::Token(t)) => t.text(self.db).as_str(self.db),
            Some(TreeToken::Branch(..)) => "branch",
        });
        res
    }
}

impl<'db> BracerIter<'db> {
    /// Get source Text and byte span for this branch, including delimiters.
    ///
    /// Returns None for top-level iterators (which have no enclosing braces).
    pub fn text_span(&self) -> Option<crate::text::TextSpan<'db>> {
        let tokens = self.tree.chunk(self.db).tokens(self.db);
        // real_token_range starts AFTER the open brace, so go back 1 for open brace.
        let open_idx = self.real_token_range.start.checked_sub(1)?;
        let close_idx = self.real_token_range.end.checked_sub(1)?;
        let open_token = tokens.get(open_idx)?;
        let close_token = tokens.get(close_idx)?;
        let text = open_token.text(self.db).text(self.db);
        let interned_text = crate::text::InternedText::new(self.db, text.text(self.db).clone());
        let span = open_token.text(self.db).range(self.db).start
                 ..close_token.text(self.db).range(self.db).end;
        Some(crate::text::TextSpan::new(interned_text, span))
    }

    fn next2(&mut self) -> Option<TreeToken<'db>> {
        loop {
            debug!("--");
            debug!("real token range {:?}", self.real_token_range.C());
            debug!("next branches {:?}", self.branches.C());
            debug!("inserted closes {:?}", self.inserted_closes.C());
            debug!("removed closes {:?}", self.removed_closes.C());
            debug!("next token index {:?}", self.next_token_index);
            debug!("next branch index {:?}", self.next_branch_index);
            debug!("next inserted close index {:?}", self.next_inserted_close_index);
            debug!("next removed close index {:?}", self.next_removed_close_index);
            debug!("--");

            let tokens = &self.tree.chunk(self.db).tokens(self.db)
                [self.real_token_range.C()];
            let branches = &self.tree.branches(self.db)
                [self.branches.C()];
            let inserted_closes = &self.tree.inserted_closes(self.db)
                [self.inserted_closes.C()];
            let removed_closes = &self.tree.removed_closes(self.db)
                [self.removed_closes.C()];
            let tokens = &self.tree.chunk(self.db).tokens(self.db)
                [0..self.real_token_range.C().end];
            let branches = &self.tree.branches(self.db)
                [0..self.branches.C().end];
            let inserted_closes = &self.tree.inserted_closes(self.db)
                [0..self.inserted_closes.C().end];
            let removed_closes = &self.tree.removed_closes(self.db)
                [0..self.removed_closes.C().end];

            let next_token = tokens.get(self.next_token_index);
            let next_branch = branches.get(self.next_branch_index);
            let next_inserted_close = inserted_closes.get(self.next_inserted_close_index);
            let next_removed_close = removed_closes.get(self.next_removed_close_index);

            return match (
                next_token,
                next_branch,
                next_inserted_close,
                next_removed_close,
            ) {
                (Some(next_token), None, _, None) => {
                    self.next_token_index = self.next_token_index.checked_add(1).X();
                    if !next_token.is_close_sigil(self.db) {
                        Some(TreeToken::Token(*next_token))
                    } else {
                        continue;
                    }
                },
                (Some(next_token), None, _, Some(next_removed_close)) => {
                    match self.next_token_index.cmp(&next_removed_close.0) {
                        Ordering::Less => {
                            self.next_token_index = self.next_token_index.checked_add(1).X();
                            if !next_token.is_close_sigil(self.db) {
                                Some(TreeToken::Token(*next_token))
                            } else {
                                panic!("is this possible?")
                            }
                        }
                        Ordering::Equal => {
                            self.next_token_index = self.next_token_index.checked_add(1).X();
                            self.next_removed_close_index = self.next_removed_close_index.checked_add(1).X();
                            continue;
                        }
                        Ordering::Greater => bug!(),
                    }
                },
                (Some(next_token), Some(next_branch), _, _) => {
                    match self.next_token_index.cmp(&next_branch.real_token_range.start) {
                        Ordering::Less => {
                            self.next_token_index = self.next_token_index.checked_add(1).X();
                            Some(TreeToken::Token(*next_token))
                        },
                        Ordering::Equal => {
                            self.next_token_index = self.next_token_index.checked_add(1).X();
                            self.next_branch_index = self.next_branch_index.checked_add(1).X();

                            // skip the opening brace of the branch
                            let branch_token_range_start = next_branch.real_token_range.start.checked_add(1).X();
                            let branch_token_range = branch_token_range_start..next_branch.real_token_range.end;

                            let branch = TreeToken::Branch(
                                next_branch.open_sigil,
                                BracerIter {
                                    db: self.db,
                                    tree: self.tree,
                                    real_token_range: branch_token_range,
                                    branches: Range::from_start_len(self.next_branch_index, next_branch.branches).X(),
                                    inserted_closes: Range::from_start_len(self.next_inserted_close_index, next_branch.inserted_closes).X(),
                                    removed_closes: Range::from_start_len(self.next_removed_close_index, next_branch.removed_closes).X(),
                                    next_token_index: self.next_token_index,
                                    next_branch_index: self.next_branch_index,
                                    next_inserted_close_index: self.next_inserted_close_index,
                                    next_removed_close_index: self.next_removed_close_index,
                                }
                            );

                            debug!("sbi {:#?}", Range::from_start_len(self.next_branch_index, next_branch.branches).X());

                            self.next_token_index = next_branch.real_token_range.end;
                            self.next_branch_index = self.next_branch_index
                                .checked_add(next_branch.branches).X();
                            self.next_inserted_close_index = self.next_inserted_close_index
                                .checked_add(next_branch.inserted_closes).X();
                            self.next_removed_close_index = self.next_removed_close_index
                                .checked_add(next_branch.removed_closes).X();

                            // Skip any removed_closes that are now behind our position.
                            // This handles cases where removed_closes exist at positions
                            // we've jumped past after exiting the branch.
                            let removed_closes = &self.tree.removed_closes(self.db)
                                [0..self.removed_closes.C().end];
                            while let Some(rc) = removed_closes.get(self.next_removed_close_index) {
                                if rc.0 < self.next_token_index {
                                    self.next_removed_close_index = self.next_removed_close_index.checked_add(1).X();
                                } else {
                                    break;
                                }
                            }

                            Some(branch)
                        },
                        Ordering::Greater => bug!(),
                    }
                }

                (None, Some(next_branch), _, _) => bug!(),

                (None, None, Some(next_inserted_close), _) => {
                    assert_eq!(next_inserted_close.0, self.next_token_index);
                    self.next_inserted_close_index = self.next_inserted_close_index.checked_add(1).X();
                    continue;
                }
                (None, None, _, Some(_next_removed_close)) => bug!(),
                (None, None, None, None) => None,
            }
        }
    }
}

#[derive(Clone)]
pub enum TreeToken<'db> {
    Token(Token<'db>),
    Branch(Sigil, BracerIter<'db>),
}

#[salsa::tracked]
pub fn bracer<'db>(
    db: &'db dyn crate::Db,
    chunk: ChunkLex<'db>
) -> Bracer<'db> {
    let tokens = chunk.tokens(db).iter().enumerate();

    #[derive(Default, Debug)]
    pub struct BraceMap {
        branches: Vec<Branch>,
        inserted_closes: Vec<(usize, Sigil)>,
        removed_closes: Vec<(usize, Sigil)>,
        errors: Vec<(Range<usize>, Sigil)>,
    }

    impl BraceMap {
        fn append(&mut self, other: BraceMap) {
            self.branches.extend(other.branches);
            self.inserted_closes.extend(other.inserted_closes);
            self.removed_closes.extend(other.removed_closes);
            self.errors.extend(other.errors);
        }
    }

    let mut top_map = BraceMap::default();
    let mut stack: Vec<(usize, Sigil, BraceMap)> = vec![];

    let mut close_brace =
        |
    stack: &mut Vec<(usize, Sigil, BraceMap)>,
    index: usize,
    open_s: Sigil,
    close_s: Sigil
        | {
            let seen_open = stack.iter().any(|(_, sigil, _)| *sigil == open_s);
            if seen_open {
                loop {
                    let (open_index, open_sigil, mut brace_map) = stack.pop().X();
                    let mut parent_brace_map = stack.last_mut()
                        .map(|(_, _, brace_map)| brace_map)
                        .unwrap_or(&mut top_map);
                    if open_sigil == open_s {
                        parent_brace_map.branches.push(Branch {
                            real_token_range: open_index..index.checked_add(1).X(),
                            branches: brace_map.branches.len(),
                            inserted_closes: brace_map.inserted_closes.len(),
                            removed_closes: brace_map.removed_closes.len(),
                            errors: brace_map.errors.len(),
                            open_sigil: open_s,
                            close_sigil: close_s,
                        });
                        parent_brace_map.append(brace_map);
                        break;
                    } else if open_sigil == Sigil::ParenOpen {
                        brace_map.inserted_closes.push((index, Sigil::ParenClose));
                        brace_map.errors.push((
                            open_index..index,
                            Sigil::ParenOpen,
                        ));
                        parent_brace_map.branches.push(Branch {
                            real_token_range: open_index..index,
                            branches: brace_map.branches.len(),
                            inserted_closes: brace_map.inserted_closes.len(),
                            removed_closes: brace_map.removed_closes.len(),
                            errors: brace_map.errors.len(),
                            open_sigil: Sigil::ParenOpen,
                            close_sigil: Sigil::ParenClose,
                        });
                        parent_brace_map.append(brace_map);
                    } else if open_sigil == Sigil::BraceOpen {
                        brace_map.inserted_closes.push((index, Sigil::BraceClose));
                        brace_map.errors.push((
                            open_index..index,
                            Sigil::BraceOpen,
                        ));
                        parent_brace_map.branches.push(Branch {
                            real_token_range: open_index..index,
                            branches: brace_map.branches.len(),
                            inserted_closes: brace_map.inserted_closes.len(),
                            removed_closes: brace_map.removed_closes.len(),
                            errors: brace_map.errors.len(),
                            open_sigil: Sigil::BraceOpen,
                            close_sigil: Sigil::BraceClose,
                        });
                        parent_brace_map.append(brace_map);
                    } else if open_sigil == Sigil::BracketOpen {
                        brace_map.inserted_closes.push((index, Sigil::BracketClose));
                        brace_map.errors.push((
                            open_index..index,
                            Sigil::BracketOpen,
                        ));
                        parent_brace_map.branches.push(Branch {
                            real_token_range: open_index..index,
                            branches: brace_map.branches.len(),
                            inserted_closes: brace_map.inserted_closes.len(),
                            removed_closes: brace_map.removed_closes.len(),
                            errors: brace_map.errors.len(),
                            open_sigil: Sigil::BracketOpen,
                            close_sigil: Sigil::BracketClose,
                        });
                        parent_brace_map.append(brace_map);
                    } else if open_sigil == Sigil::AngleOpen {
                        brace_map.inserted_closes.push((index, Sigil::AngleClose));
                        brace_map.errors.push((
                            open_index..index,
                            Sigil::AngleOpen,
                        ));
                        parent_brace_map.branches.push(Branch {
                            real_token_range: open_index..index,
                            branches: brace_map.branches.len(),
                            inserted_closes: brace_map.inserted_closes.len(),
                            removed_closes: brace_map.removed_closes.len(),
                            errors: brace_map.errors.len(),
                            open_sigil: Sigil::AngleOpen,
                            close_sigil: Sigil::AngleClose,
                        });
                        parent_brace_map.append(brace_map);
                    } else {
                        bug!()
                    }
                }
            } else {
                let mut parent_brace_map = stack.last_mut()
                    .map(|(_, _, brace_map)| brace_map)
                    .unwrap_or(&mut top_map);
                parent_brace_map.removed_closes.push((index, close_s));
                parent_brace_map.errors.push((index..index.checked_add(1).X(), close_s));
            }
        };

    for (index, token) in tokens {
        match token.kind(db) {
            TokenKind::Sigil(Sigil::ParenOpen) => {
                stack.push((index, Sigil::ParenOpen, default()));
            }
            TokenKind::Sigil(Sigil::BraceOpen) => {
                stack.push((index, Sigil::BraceOpen, default()));
            }
            TokenKind::Sigil(Sigil::BracketOpen) => {
                stack.push((index, Sigil::BracketOpen, default()));
            }
            TokenKind::Sigil(Sigil::AngleOpen) => {
                stack.push((index, Sigil::AngleOpen, default()));
            }
            TokenKind::Sigil(Sigil::ParenClose) => {
                close_brace(&mut stack, index, Sigil::ParenOpen, Sigil::ParenClose);
            }
            TokenKind::Sigil(Sigil::BraceClose) => {
                close_brace(&mut stack, index, Sigil::BraceOpen, Sigil::BraceClose);
            }
            TokenKind::Sigil(Sigil::BracketClose) => {
                close_brace(&mut stack, index, Sigil::BracketOpen, Sigil::BracketClose);
            }
            TokenKind::Sigil(Sigil::AngleClose) => {
                close_brace(&mut stack, index, Sigil::AngleOpen, Sigil::AngleClose);
            }
            _ => {},
        }
    }

    let num_tokens = chunk.tokens(db).len();

    while let Some((open_index, open_sigil, brace_map)) = stack.pop() {
        let mut parent_brace_map = stack.last_mut()
            .map(|(_, _, brace_map)| brace_map)
            .unwrap_or(&mut top_map);
        parent_brace_map.branches.push(Branch {
            real_token_range: open_index..num_tokens,
            branches: brace_map.branches.len(),
            inserted_closes: brace_map.inserted_closes.len(),
            removed_closes: brace_map.removed_closes.len(),
            errors: brace_map.errors.len(),
            open_sigil,
            close_sigil: open_sigil.close_sigil(),
        });
        parent_brace_map.errors.push((
            open_index..num_tokens,
            open_sigil,
        ));
        parent_brace_map.append(brace_map);
    }

    debug!("bm {top_map:#?}");

    Bracer::new(
        db,
        chunk,
        top_map.branches,
        top_map.inserted_closes,
        top_map.removed_closes,
        top_map.errors,
    )
}

impl<'db> TreeToken<'db> {
    /// Get source Text and byte span for this token or branch.
    pub fn text_span(&self, db: &'db dyn crate::Db) -> Option<crate::text::TextSpan<'db>> {
        match self {
            TreeToken::Token(tok) => {
                let subtext = tok.text(db);
                let text = subtext.text(db);
                let interned_text = crate::text::InternedText::new(db, text.text(db).clone());
                Some(crate::text::TextSpan::new(interned_text, subtext.range(db)))
            }
            TreeToken::Branch(_, iter) => iter.text_span(),
        }
    }

    pub fn without_space(self, db: &'db dyn crate::Db) -> Option<Self> {
        match self {
            TreeToken::Token(token) => {
                token.without_space(db)
                    .map(TreeToken::Token)
            }
            token @ TreeToken::Branch(..) => Some(token),
        }
    }
}

#[cfg(test)]
#[extension_trait]
impl<'db> VecTreeTokenExt<'db> for Vec<TreeToken<'db>> {
    fn debug_str(&self, db: &'db dyn crate::Db) -> String {
        let mut buf = Vec::<u8>::new();
        Bracer::debug_write(self.iter().cloned(), &mut buf, db).X();
        String::from_utf8(buf).X()
    }
}

#[cfg(test)]
#[extension_trait]
pub impl<'db, I> IteratorOfTreeTokenExt<'db> for I
where I: Iterator<Item = TreeToken<'db>>
{
    fn debug_str(self, db: &'db dyn crate::Db) -> String {
        let mut buf = Vec::<u8>::new();
        Bracer::debug_write(self, &mut buf, db).X();
        String::from_utf8(buf).X()
    }
}

#[cfg(test)]
impl<'db> Bracer<'db> {
    fn debug_str(&self, db: &'db dyn crate::Db) -> String {
        let mut buf = Vec::<u8>::new();
        Self::debug_write(self.iter(db), &mut buf, db).X();
        String::from_utf8(buf).X()
    }

    fn debug_write(
        iter: impl Iterator<Item = TreeToken<'db>>,
        w: &mut dyn Write,
        db: &'db dyn crate::Db,
    ) -> AnyResult<()> {
        let mut iter = iter.peekable();
        while let Some(token) = iter.next() {
            match token {
                TreeToken::Token(token) => {
                    write!(w, "{}", token.debug_str(db))?;
                }
                TreeToken::Branch(sigil, mut next_iter) => {
                    write!(w, "{} ", sigil.as_str())?;
                    rmx::extras::recurse(|| {
                        Self::debug_write(next_iter.C(), w, db)
                    })?;
                    if next_iter.next().is_some() {
                        write!(w, " ");
                    }
                    write!(w, "{}", sigil.close_sigil().as_str())?;
                }
            }
            if iter.peek().is_some() {
                write!(w, " ")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
fn dbglex(s: &str) -> String {
    debug!("dbglex {s}");
    let ref db = crate::Database::default();
    let source = crate::input::Source::new(db, S(s));
    let chunk = crate::source_map::basic_source_map(db, source);
    let chunk_lex = crate::lexer::lex_chunk(db, chunk);
    let bracer = bracer(db, chunk_lex);
    bracer.debug_str(db)
}

#[test]
fn test_bracer() {
    assert_eq!(
        dbglex(" "),
        "ws",
    );
    assert_eq!(
        dbglex("a b"),
        "a ws b",
    );
    assert_eq!(
        dbglex("a\nb"),
        "a ws b",
    );
    assert_eq!(
        dbglex("()"),
        "( )",
    );
    assert_eq!(
        dbglex("{}"),
        "{ }",
    );
    assert_eq!(
        dbglex("())"),
        "( )",
    );
    assert_eq!(
        dbglex("(})"),
        "( )",
    );
    assert_eq!(
        dbglex("(()"),
        "( ( ) )",
    );
    assert_eq!(
        dbglex("({)"),
        "( { } )",
    );
    assert_eq!(
        dbglex(")"),
        "",
    );
    assert_eq!(
        dbglex("))})"),
        "",
    );
    assert_eq!(
        dbglex("(({("),
        "( ( { ( ) } ) )",
    );
    assert_eq!(
        dbglex("a(b)c"),
        "a ( b ) c",
    );
    assert_eq!(
        dbglex("a(b(c"),
        "a ( b ( c ) )",
    );
    assert_eq!(
        dbglex("a)b)c"),
        "a b c",
    );
    assert_eq!(
        dbglex("(a}b}c)"),
        "( a b c )",
    );
    assert_eq!(
        dbglex("[]"),
        "[ ]",
    );
    assert_eq!(
        dbglex("<>"),
        "< >",
    );
    assert_eq!(
        dbglex("a[b]c"),
        "a [ b ] c",
    );
    assert_eq!(
        dbglex("a<b>c"),
        "a < b > c",
    );
    assert_eq!(
        dbglex("([{<>}])"),
        "( [ { < > } ] )",
    );
    // Mismatch: paren inside brace closed by brace.
    assert_eq!(
        dbglex("{(}"),
        "{ ( ) }",
    );
    // Mismatch: bracket inside brace closed by brace.
    assert_eq!(
        dbglex("{[}"),
        "{ [ ] }",
    );
    // Mismatch: angle inside brace closed by brace.
    assert_eq!(
        dbglex("{<}"),
        "{ < > }",
    );
    // Mismatch: bracket inside paren closed by paren.
    assert_eq!(
        dbglex("([)"),
        "( [ ] )",
    );
    // Mismatch: angle inside paren closed by paren.
    assert_eq!(
        dbglex("(<)"),
        "( < > )",
    );
    // Mismatch: angle inside bracket closed by bracket.
    assert_eq!(
        dbglex("[<]"),
        "[ < > ]",
    );
}

#[test]
fn test_text_span() {
    let ref db = crate::Database::default();

    // Helper to get the span string from input.
    let get_span = |s: &str| -> Option<(usize, usize, String)> {
        let source = crate::input::Source::new(db, S(s));
        let chunk = crate::source_map::basic_source_map(db, source);
        let chunk_lex = crate::lexer::lex_chunk(db, chunk);
        let bracer = bracer(db, chunk_lex);
        // Find the first branch.
        for token in bracer.iter(db) {
            if let TreeToken::Branch(_, _) = &token {
                let ts = token.text_span(db)?;
                let spanned = &ts.text.as_str(db)[ts.span.clone()];
                return Some((ts.start(), ts.end(), spanned.to_string()));
            }
        }
        None
    };

    // Simple branch - span covers entire (a).
    let (start, end, spanned) = get_span("(a)").X();
    assert_eq!(spanned, "(a)");
    assert_eq!(start, 0);
    assert_eq!(end, 3);

    // Empty branch - span covers ().
    let (start, end, spanned) = get_span("()").X();
    assert_eq!(spanned, "()");
    assert_eq!(start, 0);
    assert_eq!(end, 2);

    // Branch with leading content.
    let (start, end, spanned) = get_span("x(a)").X();
    assert_eq!(spanned, "(a)");
    assert_eq!(start, 1);
    assert_eq!(end, 4);

    // Unclosed branch.
    let (start, end, spanned) = get_span("(a").X();
    assert_eq!(spanned, "(a");
    assert_eq!(start, 0);
    assert_eq!(end, 2);

    // Nested branches - outer.
    let (start, end, spanned) = get_span("((a))").X();
    assert_eq!(spanned, "((a))");
    assert_eq!(start, 0);
    assert_eq!(end, 5);

    // Different bracket types.
    let (start, end, spanned) = get_span("[x]").X();
    assert_eq!(spanned, "[x]");
    assert_eq!(start, 0);
    assert_eq!(end, 3);

    let (start, end, spanned) = get_span("{y}").X();
    assert_eq!(spanned, "{y}");
    assert_eq!(start, 0);
    assert_eq!(end, 3);

    let (start, end, spanned) = get_span("<z>").X();
    assert_eq!(spanned, "<z>");
    assert_eq!(start, 0);
    assert_eq!(end, 3);
}

#[test]
fn test_without_space() {
    let ref db = crate::Database::default();
    let source = crate::input::Source::new(db, S("a b (c)"));
    let chunk = crate::source_map::basic_source_map(db, source);
    let chunk_lex = crate::lexer::lex_chunk(db, chunk);
    let bracer = bracer(db, chunk_lex);

    let tokens: Vec<_> = bracer.iter(db).collect();
    // tokens: "a", ws, "b", ws, branch(c)
    assert_eq!(tokens.len(), 5);

    // Token "a" - not whitespace, returns Some.
    let t0 = tokens[0].clone().without_space(db);
    assert!(t0.is_some());

    // Whitespace token - returns None.
    let t1 = tokens[1].clone().without_space(db);
    assert!(t1.is_none());

    // Branch - always returns Some.
    let t4 = tokens[4].clone().without_space(db);
    assert!(t4.is_some());
}

#[test]
fn test_removed_closes() {
    // Stray closes get removed.
    assert_eq!(dbglex("a)b"), "a b");
    assert_eq!(dbglex("a}b"), "a b");
    assert_eq!(dbglex("a]b"), "a b");
    assert_eq!(dbglex("a>b"), "a b");
    // Multiple stray closes.
    assert_eq!(dbglex("a)}]>b"), "a b");
    // Stray close inside matched braces.
    assert_eq!(dbglex("(a}b)"), "( a b )");
    assert_eq!(dbglex("(a}b}c)"), "( a b c )");
    // Stray closes after matched braces.
    assert_eq!(dbglex("(a))"), "( a )");
    assert_eq!(dbglex("(a)})"), "( a )");
    // Complex nesting with stray closes.
    assert_eq!(dbglex("((a)})"), "( ( a ) )");
}
