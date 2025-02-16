use rmx::prelude::*;

use rmx::itertools::Itertools;
use rmx::std::ops::Range;
use rmx::std::{iter, mem};
use rmx::std::collections::BTreeMap;

use crate::input::Source;
use crate::text::{Text, SubText};
use crate::chunk::{Chunk, RangeKind};
use crate::source_map::{
    basic_source_map,
};

#[salsa::tracked]
pub struct ChunkLex<'db> {
    chunk: Chunk<'db>,
    #[return_ref]
    pub tokens: Vec<Token<'db>>,
}

#[salsa::tracked]
pub struct Token<'db> {
    pub text: SubText<'db>,
    pub kind: TokenKind,
}

#[derive(Copy, Clone, Debug, Hash, salsa::Update)]
#[derive(Eq, PartialEq)]
pub enum TokenKind {
    Word,
    Sigil(Sigil),
    String,
    Whitespace,
    Comment,
    Error,
}

#[derive(Copy, Clone, Debug, Hash, salsa::Update)]
#[derive(Eq, PartialEq)]
#[derive(enum_iterator::Sequence)]
pub enum Sigil {
    Dot,
    Comma,
    Semicolon,
    ColonDash,
    ParenOpen,
    ParenClose,
    BraceOpen,
    BraceClose,
}

#[salsa::tracked]
pub fn lex_chunk<'db>(
    db: &'db dyn crate::Db,
    chunk: Chunk<'db>,
) -> ChunkLex<'db> {
    let mut tokens = Vec::new();
    let chunk_text = chunk.text(db);

    for range in chunk.ranges(db) {
        match range {
            (range, RangeKind::Comment) => {
                tokens.push(Token::new(
                    db,
                    chunk_text.sub(db, range),
                    TokenKind::Comment,
                ));
            }
            (range, RangeKind::String) => {
                tokens.push(Token::new(
                    db,
                    chunk_text.sub(db, range),
                    TokenKind::String,
                ));
            }
            (range, RangeKind::Error) => {
                tokens.push(Token::new(
                    db,
                    chunk_text.sub(db, range),
                    TokenKind::Error,
                ));
            }
            (range, RangeKind::Unknown) => {
                let mut tokenizer = Tokenizer {
                    db,
                    chunk,
                    range,
                    chunk_text: chunk_text.C(),
                };

                tokens.extend(
                    iter::from_fn(|| tokenizer.next())
                );
            }
        }
    }

    return ChunkLex::new(db, chunk, tokens);

    struct Tokenizer<'db> {
        db: &'db dyn crate::Db,
        chunk: Chunk<'db>,
        chunk_text: Text<'db>,
        range: Range<usize>,
    }

    #[derive(Eq, PartialEq, Debug, Copy, Clone)]
    enum NextToken {
        Whitespace,
        Word,
        Sigil,
        Error,
    }

    impl<'db> Tokenizer<'db> {
        fn next(&mut self) -> Option<Token<'db>> {
            match self.peek_token() {
                None => None,
                Some(NextToken::Whitespace) => Some(self.eat_whitespace()),
                Some(NextToken::Word) => Some(self.eat_word()),
                Some(NextToken::Sigil) => Some(self.eat_sigil()),
                Some(NextToken::Error) => Some(self.eat_error()),
            }
        }

        fn peek_token(&self) -> Option<NextToken> {
            self.peek().map(Self::token_start)
        }

        fn token_start(ch: char) -> NextToken {
            match ch {
                _ if ch.is_whitespace() => NextToken::Whitespace,
                _ if Self::is_word_start(ch) => NextToken::Word,
                _ if Self::is_sigil_start(ch) => NextToken::Sigil,
                _ => NextToken::Error,
            }
        }

        fn is_word_start(ch: char) -> bool {
            ch.is_alphanumeric() || ch == '_'
        }

        fn eat_word(&mut self) -> Token<'db> {
            assert_eq!(self.peek_token(), Some(NextToken::Word));

            let is_word_char = Self::is_word_start;

            let start = self.range.start;
            while let Some(ch) = self.peek() {
                if is_word_char(ch) {
                    self.eat_char(ch);
                } else {
                    break;
                }
            }
            assert!(start < self.range.start);
            Token::new(
                self.db,
                self.chunk_text.sub(self.db, start .. self.range.start),
                TokenKind::Word,
            )
        }

        fn is_sigil_start(ch: char) -> bool {
            enum_iterator::all::<Sigil>().map(|s| s.start_char()).any(|c| c == ch)
        }

        fn eat_sigil(&mut self) -> Token<'db> {
            assert_eq!(self.peek_token(), Some(NextToken::Sigil));

            let all_sigils = enum_iterator::all::<Sigil>();
            let text = &self.chunk.text(self.db).as_str(self.db)[self.range.C()];

            for sigil in all_sigils {
                let sigil_str = sigil.as_str();
                if text.starts_with(sigil_str) {
                    let range_start = self.range.start;
                    self.range.start = range_start.checked_add(sigil_str.len()).X();
                    return Token::new(
                        self.db,
                        self.chunk_text.sub(self.db, range_start .. self.range.start),
                        TokenKind::Sigil(sigil),
                    )
                    
                }
            }

            self.eat_error_from(self.peek().X())
        }

        fn eat_error(&mut self) -> Token<'db> {
            self.eat_error_from(self.peek().X())
        }

        fn eat_error_from(&mut self, start_ch: char) -> Token<'db> {
            assert_eq!(self.peek_token(), Some(NextToken::Whitespace));

            let token_start = Self::token_start(start_ch);
            let start = self.range.start.checked_sub(1).X();
            while let Some(ch) = self.peek() {
                let next_token_start = Self::token_start(ch);
                let recover = match (token_start, next_token_start) {
                    (NextToken::Whitespace, _) => unreachable!(),
                    (NextToken::Word, _) => unreachable!(),
                    (NextToken::Sigil, NextToken::Error) => false,
                    (NextToken::Sigil, NextToken::Whitespace) => true,
                    (NextToken::Sigil, NextToken::Word) => true,
                    (NextToken::Sigil, NextToken::Sigil) => true,
                    (NextToken::Error, NextToken::Error) => false,
                    (NextToken::Error, NextToken::Whitespace) => true,
                    (NextToken::Error, NextToken::Word) => true,
                    (NextToken::Error, NextToken::Sigil) => true,
                };
                if !recover {
                    self.eat_char(ch);
                } else {
                    break;
                }
            }
            assert!(start < self.range.start);
            Token::new(
                self.db,
                self.chunk_text.sub(self.db, start .. self.range.start),
                TokenKind::Error,
            )
        }

        fn eat_whitespace(&mut self) -> Token<'db> {
            assert_eq!(self.peek_token(), Some(NextToken::Whitespace));

            let start = self.range.start;
            while let Some(ch) = self.peek() {
                if ch.is_whitespace() {
                    self.eat_char(ch);
                } else {
                    break;
                }
            }
            assert!(start < self.range.start);
            Token::new(
                self.db,
                self.chunk_text.sub(self.db, start .. self.range.start),
                TokenKind::Whitespace,
            )
        }

        fn eat_char(&mut self, ch: char) {
            assert!(self.peek() == Some(ch));
            self.range.start = self.range.start.checked_add(ch.len_utf8()).X();
            assert!(self.range.start <= self.range.end);
        }

        fn peek(&self) -> Option<char> {
            self.chunk.text(self.db).as_str(self.db)[self.range.C()].chars().next()
        }
    }
}

impl<'db> ChunkLex<'db> {
    #[cfg(test)]
    fn debug_str(&self, db: &'db dyn crate::Db) -> String {
        #[allow(unstable_name_collisions)] // intersperse
        self.tokens(db).iter().map(|token| {
            token.debug_str(db)
        }).intersperse(" ").collect()
    }
}

impl<'db> Token<'db> {
    #[cfg(test)]
    pub fn debug_str(&self, db: &'db dyn crate::Db) -> &'db str {
        match self.kind(db) {
            TokenKind::Word | TokenKind::String => {
                self.text(db).as_str(db)
            }
            TokenKind::Sigil(s) => s.as_str(),
            TokenKind::Whitespace => "ws",
            TokenKind::Comment => "cmt",
            TokenKind::Error => "err",
        }
    }

    pub fn is_close_sigil(&self, db: &'db dyn crate::Db) -> bool {
        match self.kind(db) {
            TokenKind::Sigil(s) => s.is_close_sigil(),
            _ => false,
        }
    }
}

impl Sigil {
    pub fn as_str(&self) -> &'static str {
        match self {
            Sigil::Dot => ".",
            Sigil::Comma => ",",
            Sigil::Semicolon => ";",
            Sigil::ColonDash => ":-",
            Sigil::ParenOpen => "(",
            Sigil::ParenClose => ")",
            Sigil::BraceOpen => "{",
            Sigil::BraceClose => "}",
        }
    }

    fn start_char(&self) -> char {
        self.as_str().chars().next().X()
    }

    pub fn close_sigil(&self) -> Sigil {
        match self {
            Sigil::ParenOpen => Sigil::ParenClose,
            Sigil::BraceOpen => Sigil::BraceClose,
            _ => bug!(),
        }
    }

    fn is_close_sigil(&self) -> bool {
        matches!(self, Sigil::ParenClose | Sigil::BraceClose)
    }
}

#[test]
fn test_lex_chunk() {
    fn dbglex(s: &str) -> String {
        let ref db = crate::Database::default();
        let source = Source::new(db, S(s));
        let chunk = basic_source_map(db, source);
        let chunk_lex = lex_chunk(db, chunk);
        chunk_lex.debug_str(db)
    }

    assert_eq!(
        dbglex(" "),
        "ws",
    );
    assert_eq!(
        dbglex("a"),
        "a",
    );
    assert_eq!(
        dbglex("a b"),
        "a ws b",
    );
    assert_eq!(
        dbglex("a:-b"),
        "a :- b",
    );
    assert_eq!(
        dbglex("a :- b \n c"),
        "a ws :- ws b ws c",
    );
    assert_eq!(
        dbglex("a%"),
        "a cmt",
    );
    assert_eq!(
        dbglex("a%\n"),
        "a cmt ws",
    );
    assert_eq!(
        dbglex("a%\nd"),
        "a cmt ws d",
    );
    assert_eq!(
        dbglex("(){}){"),
        "( ) { } ) {",
    );
}


