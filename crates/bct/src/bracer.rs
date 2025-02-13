use rmx::prelude::*;
use rmx::core::ops::Range;
use rmx::core::iter::Peekable;
use rmx::std::io::Write;

use crate::chunk::Chunk;
use crate::lexer::{ChunkLex, Token, TokenKind, Sigil};

#[salsa::tracked]
pub struct Bracer<'db> {
    pub chunk: ChunkLex<'db>,
    #[return_ref]
    pub branches: Vec<Branch>,
    #[return_ref]
    pub inserted_closes: Vec<(usize, Sigil)>,
    #[return_ref]
    pub removed_closes: Vec<(usize, Sigil)>,
    #[return_ref]
    pub errors: Vec<(Range<usize>, Sigil)>,
}

#[derive(Clone, Debug, salsa::Update)]
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
    db: &'db dyn crate::Db,
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
                                continue;
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
                            open_index..index,//.checked_add(1).X(),
                            Sigil::ParenOpen,
                        ));
                        parent_brace_map.branches.push(Branch {
                            real_token_range: open_index..index,//index.checked_add(1).X(),
                            branches: brace_map.branches.len(),
                            inserted_closes: brace_map.inserted_closes.len(),
                            removed_closes: brace_map.removed_closes.len(),
                            errors: brace_map.errors.len(),
                            open_sigil: Sigil::ParenOpen,
                            close_sigil: Sigil::ParenClose,
                        });
                        // parent_brace_map.inserted_closes.push((index, Sigil::ParenClose));
                        // parent_brace_map.errors.push((
                        //     open_index..index.checked_add(1).X(),
                        //     Sigil::ParenOpen,
                        // ));
                        parent_brace_map.append(brace_map);
                    } else if open_sigil == Sigil::BraceOpen {
                        brace_map.inserted_closes.push((index, Sigil::BraceClose));
                        brace_map.errors.push((
                            open_index..index,//.checked_add(1).X(),
                            Sigil::BraceOpen,
                        ));
                        parent_brace_map.branches.push(Branch {
                            real_token_range: open_index..index,//checked_add(1).X(),
                            branches: brace_map.branches.len(),
                            inserted_closes: brace_map.inserted_closes.len(),
                            removed_closes: brace_map.removed_closes.len(),
                            errors: brace_map.errors.len(),
                            open_sigil: Sigil::BraceOpen,
                            close_sigil: Sigil::BraceClose,
                        });
                        // parent_brace_map.inserted_closes.push((index, Sigil::BraceClose));
                        // parent_brace_map.errors.push((
                        //     open_index..index.checked_add(1).X(),
                        //     Sigil::BraceOpen,
                        // ));
                        parent_brace_map.append(brace_map);
                    } else {
                        todo!()
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
            TokenKind::Sigil(Sigil::ParenClose) => {
                close_brace(&mut stack, index, Sigil::ParenOpen, Sigil::ParenClose);
            }
            TokenKind::Sigil(Sigil::BraceClose) => {
                close_brace(&mut stack, index, Sigil::BraceOpen, Sigil::BraceClose);
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
            open_sigil: open_sigil,
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

#[cfg(test)]
impl<'db> Bracer<'db> {
    fn debug_str(&self, db: &'db dyn crate::Db) -> String {
        let mut buf = Vec::<u8>::new();
        self.debug_write(&mut buf, db, self.iter(db)).X();
        String::from_utf8(buf).X()
    }

    fn debug_write(
        &self,
        w: &mut dyn Write,
        db: &'db dyn crate::Db,
        iter: BracerIter<'db>,
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
                        self.debug_write(w, db, next_iter.C())
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
    let db = &crate::Database::default();
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
}
