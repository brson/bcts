use rmx::prelude::*;
use rmx::core::iter;

use crate::bracer::{Bracer, TreeToken};
use crate::lexer::{Token, TokenKind};

impl<'db> Bracer<'db> {
    fn lines(&self, db: &'db dyn crate::Db) -> impl Iterator<Item = impl Iterator<Item = TreeToken<'db>>> {
        self.iter(db).batching(|iter| {
            iter.next().map(|token| {
                Some(token).into_iter().chain(iter.clone()).scan(false, |stop, next_token| {
                    if *stop {
                        None
                    } else {
                        if next_token.is_whitespace_newline(db) {
                            *stop = true;
                        }

                        Some(next_token)
                    }
                })
            })
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
    bracer.lines(db).map(|mut line| (&mut line as &mut dyn Iterator<Item = TreeToken>).debug_str(db)).collect()
}
