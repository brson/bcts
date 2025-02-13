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
