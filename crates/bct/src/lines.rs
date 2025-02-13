use rmx::prelude::*;

use rmx::core::iter;
use rmx::core::ops::Range;

use crate::bracer::{Bracer, TreeToken};
use crate::lexer::{Token, TokenKind};

#[salsa::tracked]
pub struct Lines<'db> {
    bracer: Bracer<'db>,
    #[return_ref]
    lines: Vec<Range<usize>>,
}

impl<'db> Bracer<'db> {
    fn lines(&self, db: &'db dyn crate::Db) -> impl Iterator<Item = impl Iterator<Item = TreeToken<'db>>> {
        self.iter(db).peekable().batching(|iter| {
            // fixme big allocs
            let mut buf = vec![];
            for token in iter {
                if token.is_whitespace_newline(db) {
                    buf.push(token);
                    break;
                } else {
                    buf.push(token);
                }
            }
            if buf.is_empty() {
                None
            } else {
                Some(buf.into_iter())
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
    bracer.lines(db).map(|mut line| line.debug_str(db)).collect()
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
        dbglex("a\nb"),
        vec![S("a ws"), S("b")],
    );
}
