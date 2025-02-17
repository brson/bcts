use rmx::prelude::*;

use rmx::core::iter;
use rmx::core::ops::Range;

use crate::bracer::{Bracer, TreeToken, BracerIter};
use crate::lexer::{Token, TokenKind};

impl<'db> BracerIter<'db> {
    pub fn lines(self) -> impl Iterator<Item = impl Iterator<Item = TreeToken<'db>>> {
        let db = self.db;
        self.batching(move |iter| {
            let mut iter_clone = iter.C().peekable();
            while let Some(token) = iter.next() {
                if token.is_whitespace_newline(db) {
                    break;
                }
            }
            if iter_clone.peek().is_some() {
                Some(iter_clone.scan(false, move |stop, token| {
                    if *stop {
                        None
                    } else {
                        if token.is_whitespace_newline(db) {
                            *stop = true;
                        }
                        Some(token)
                    }
                }))
            } else {
                None
            }
        })
    }
}

impl<'db> TreeToken<'db> {
    fn is_whitespace_newline(&self, db: &'db dyn crate::Db) -> bool {
        return match self {
            TreeToken::Token(token) => {
                is_whitespace_newline(db, *token)
            }
            TreeToken::Branch(..) => false,
        };

        #[salsa::tracked]
        pub fn is_whitespace_newline<'db>(
            db: &'db dyn crate::Db,
            token: Token<'db>
        ) -> bool {
            match token.kind(db) {
                TokenKind::Whitespace => {
                    token.text(db).as_str(db).contains("\n")
                }
                _ => false,
            }
        }
    }
}

#[cfg(test)]
fn dbglex(s: &str) -> Vec<String> {
    debug!("dbglex {s}");
    use crate::bracer::IteratorOfTreeTokenExt as _;
    let ref db = crate::Database::default();
    let source = crate::input::Source::new(db, S(s));
    let chunk = crate::source_map::basic_source_map(db, source);
    let chunk_lex = crate::lexer::lex_chunk(db, chunk);
    let bracer = crate::bracer::bracer(db, chunk_lex);
    bracer.iter(db).lines().map(|mut line| line.debug_str(db)).collect()
}

#[test]
fn test_lines() {
    assert_eq!(
        dbglex(""),
        Vec::<String>::new(),
    );
    assert_eq!(
        dbglex(" "),
        vec![S("ws")],
    );
    assert_eq!(
        dbglex("\n"),
        vec![S("ws")],
    );
    assert_eq!(
        dbglex("a\n"),
        vec![S("a ws")],
    );
    assert_eq!(
        dbglex("a\nb"),
        vec![S("a ws"), S("b")],
    );
    assert_eq!(
        dbglex("a\nb\n"),
        vec![S("a ws"), S("b ws")],
    );
    assert_eq!(
        dbglex("a(b\nc)d"),
        vec![S("a ( b ws c ) d")],
    );
    assert_eq!(
        dbglex("a(b\nc)d\na(b\nc)d"),
        vec![S("a ( b ws c ) d ws"), S("a ( b ws c ) d")],
    );
}
