use rmx::prelude::*;
use rmx::std::ops::Range;
use rmx::std::{iter, mem};
use rmx::std::iter::Peekable;
use rmx::std::slice::Iter as SliceIter;

use crate::text::Text;
use crate::chunk::{Chunk, RangeKind};

#[salsa::tracked]
pub struct Chunks<'db> {
    #[return_ref]
    pub chunks: Vec<Chunk<'db>>,
}

#[salsa::tracked]
pub fn basic_chunks<'db>(
    db: &'db dyn crate::Db,
    chunk_in: Chunk<'db>
) -> Chunks<'db> {
    chunks(
        db,
        chunk_in,
        basic_config(db),
    )
}

#[salsa::tracked]
pub struct Config<'db> {
    #[return_ref]
    chunk_start_chars: Vec<char>,
    try_chunk: for <'a> fn(&'a str) -> Option<usize>,
}

#[salsa::tracked]
pub fn basic_config<'db>(
    db: &'db dyn crate::Db,
) -> Config<'db> {
    Config::new(
        db,
        vec!['.'],
        basic_try_chunk,
    )
}

fn basic_try_chunk(text: &str) -> Option<usize> {
    assert_eq!(text.as_bytes()[0], b'.');
    Some(1)
}

#[salsa::tracked]
pub fn chunks<'db>(
    db: &'db dyn crate::Db,
    chunk_in: Chunk<'db>,
    config: Config<'db>,
) -> Chunks<'db> {
    let mut state = State {
        db,
        config,
        chunk_in,
        comments_iter: chunk_in.comments(db).iter().peekable(),
        strings_iter: chunk_in.strings(db).iter().peekable(),
        errors_iter: chunk_in.errors(db).iter().peekable(),
        position: 0,
        chunk_wip: ChunkWip {
            chunk_start: 0,
            comments: vec![],
            strings: vec![],
            errors: vec![],
        },
        chunks: vec![],
    };

    state.map()
}

struct State<'db> {
    db: &'db dyn crate::Db,
    config: Config<'db>,
    chunk_in: Chunk<'db>,
    comments_iter: Peekable<SliceIter<'db, Range<usize>>>,
    strings_iter: Peekable<SliceIter<'db, Range<usize>>>,
    errors_iter: Peekable<SliceIter<'db, Range<usize>>>,
    position: usize,
    chunk_wip: ChunkWip,
    chunks: Vec<Chunk<'db>>,
}

// ranges are relative to `chunk_start`
struct ChunkWip {
    chunk_start: usize,
    comments: Vec<Range<usize>>,
    strings: Vec<Range<usize>>,
    errors: Vec<Range<usize>>,
}

impl<'db> State<'db> {
    fn map(mut self) -> Chunks<'db> {
        let all_start_chars = &**self.config.chunk_start_chars(self.db);
        let text_all = self.chunk_in.text(self.db).as_str(self.db);

        for (range, kind) in self.chunk_in.ranges(self.db) {
            if !matches!(kind, RangeKind::Unknown) {
                continue;
            }

            self.position = range.start;

            loop {
                let text_remaining = &text_all[self.position..range.end];
                let mut start_char_indexes = text_remaining.match_indices(all_start_chars).map(|(i, s)| i);
                let next_start_char_index = start_char_indexes.next();

                match next_start_char_index {
                    Some(start_char_index) => {
                        self.position = self.position.checked_add(start_char_index).X();
                        let text_remaining = &text_remaining[start_char_index..];

                        let try_chunk_res = self.try_chunk(text_remaining);

                        self.step(
                            try_chunk_res,
                        );
                    }
                    None => {
                        self.position = range.end;

                        break;
                    }
                }
            }
        }

        let text_remaining = &text_all[self.position..];
        self.push_chunk(text_remaining.len());

        assert_eq!(self.position, self.chunk_wip.chunk_start);
        assert!(self.chunk_wip.comments.is_empty());
        assert!(self.chunk_wip.strings.is_empty());
        assert!(self.chunk_wip.errors.is_empty());

        Chunks::new(
            self.db,
            self.chunks,
        )
    }

    fn step(
        &mut self,
        try_chunk: Option<usize>,
    ) {
        let chunk_offset = self.position.checked_sub(self.chunk_wip.chunk_start).X();

        match try_chunk {
            Some(chunk_extra_bytes) => {
                self.push_chunk(chunk_extra_bytes);
            }
            None => {
                self.position = self.position.checked_add(1).X();
                let text_all = self.chunk_in.text(self.db).as_str(self.db);
                assert!(self.position <= text_all.len());
            }
        }
    }

    fn push_chunk(&mut self, eat_bytes: usize) {
        self.position = self.position.checked_add(eat_bytes).X();
        self.collect_ranges();
        let text_all = self.chunk_in.text(self.db).as_str(self.db);
        assert!(self.position <= text_all.len());
        let chunk_text = &text_all[self.chunk_wip.chunk_start..self.position];
        if !chunk_text.is_empty() {
            self.chunks.push(
                Chunk::new(
                    self.db,
                    Text::new(self.db, S(chunk_text)),
                    mem::take(&mut self.chunk_wip.comments),
                    mem::take(&mut self.chunk_wip.strings),
                    mem::take(&mut self.chunk_wip.errors),
                )
            );
        }
        self.chunk_wip.chunk_start = self.position;
    }

    fn try_chunk(&self, text: &str) -> Option<usize> {
        let start_char = text.chars().next().X();
        if self.config.chunk_start_chars(self.db).contains(&start_char) {
            self.config.try_chunk(self.db)(text)
        } else {
            None
        }
    }

    fn collect_ranges(&mut self) {
        let configs = [
            (&mut self.comments_iter,
             &mut self.chunk_wip.comments),
            (&mut self.strings_iter,
             &mut self.chunk_wip.strings),
            (&mut self.errors_iter,
             &mut self.chunk_wip.errors),
        ];
        for (iter, vec) in configs {
            while let Some(range) = iter.peek() {
                if range.start >= self.position {
                    break;
                }
                assert!(range.end <= self.position);
                vec.push(
                    iter.next().X().clone().checked_sub(self.chunk_wip.chunk_start).expect("poo")
                );
            }
        }
    }
}

#[test]
fn test_source_map() {
    fn chunk<'db>(db: &'db dyn crate::Db, s: &str) -> Chunks<'db> {
        use crate::input::Source;
        use crate::source_map::basic_source_map;

        let source = Source::new(db, S(s));
        let full_chunk = basic_source_map(db, source);
        basic_chunks(db, full_chunk)
    }

    // "F"ragment
    #[derive(Debug, Copy, Clone)]
    enum F<'s> {
        T(&'s str),
        C(&'s str),
        S(&'s str),
        E(&'s str),
        Dot,
    }

    fn strs<'s, 'ss>(frags: &'ss [F<'s>]) -> impl Iterator<Item = &'s str> + 'ss {
        frags.iter().map(|f| match f {
            F::T(s) | F::C(s) | F::S(s) | F::E(s) => s,
            F::Dot => ".",
        })
    }

    fn source(frags: &[F<'_>]) -> String {
        strs(frags).collect()
    }

    fn positions<'ss>(frags: &'ss [F<'_>]) -> impl Iterator<Item = usize> + 'ss {
        strs(frags).scan(0, |pos, s| {
            let next = *pos;
            *pos = next + s.len();
            Some(next)
        })
    }

    fn run<'s>(frags: &[F<'s>]) {
        eprintln!("FS '{frags:?}'");
        let db = &crate::Database::default();
        let text = source(frags);
        eprintln!("t '{text}'");
        let map = chunk(db, &text);

        // Grouping on dots
        let ex_chunks = frags.iter().peekable()
            .batching(|iter| {
                if iter.peek().is_some() {
                    Some(iter.take_while_inclusive(|frag| {
                        !matches!(frag, F::Dot)
                    }).collect::<Vec<_>>())
                } else {
                    None
                }
            });

        for (ex_chunk, a_chunk) in ex_chunks.zip(map.chunks(db).iter()) {
            let ex_frags = ex_chunk.iter().map(|f| **f).collect::<Vec<_>>();
            let ex_text = source(&ex_frags);
            let ex_positions = positions(&ex_frags);
            let ex_chunk = ex_chunk.iter().zip(ex_positions);
            let ex_chunk = ex_chunk.collect::<Vec<_>>();

            let a_text = a_chunk.text(db).as_str(db);
            assert_eq!(&ex_text, a_text);
            eprintln!("at '{a_text}'");

            let mut comments = a_chunk.comments(db).C();
            let mut strings = a_chunk.strings(db).C();
            let mut errors = a_chunk.errors(db).C();
            for (frag, pos) in ex_chunk.iter().rev() {
                eprintln!("f {frag:?} {pos}");
                match frag {
                    F::T(s) => {
                        assert_eq!(&a_text[*pos..][..s.len()], *s);
                    }
                    F::C(s) => {
                        assert_eq!(&a_text[*pos..][..s.len()], *s);
                        let range = comments.pop().X();
                        assert_eq!(range, *pos .. (*pos + s.len()));
                    }
                    F::S(s) => {
                        assert_eq!(&a_text[*pos..][..s.len()], *s);
                        let range = strings.pop().X();
                        assert_eq!(range, *pos .. (*pos + s.len()));
                    }
                    F::E(s) => {
                        assert_eq!(&a_text[*pos..][..s.len()], *s);
                        let range = errors.pop().X();
                        assert_eq!(range, *pos .. (*pos + s.len()));
                    }
                    F::Dot => {
                        assert_eq!(&a_text[*pos..][..1], ".");
                    }
                }
            }

            assert!(comments.is_empty());
            assert!(strings.is_empty());
            assert!(errors.is_empty());
        }
    }

    run(&[
        F::T("ab"),
        F::Dot,
        F::T("bdd"),
        F::C("%"),
    ]);
    run(&[
        F::T("ab"),
        F::Dot,
        F::T("bdd"),
        F::C("%"),
        F::T("\n"),
    ]);
    run(&[
        F::T("ab"),
        F::Dot,
        F::C("%a"),
        F::T("\nbdd"),
        F::C("%b"),
        F::T("\n"),
        F::C("%b"),
    ]);
    run(&[
        F::T("ab"),
        F::Dot,
        F::T("bdd"),
        F::S("\"x\""),
    ]);
    run(&[
        F::T("ab"),
        F::Dot,
        F::T("bdd"),
        F::S("\"x\""),
        F::S("\"x\""),
    ]);
    run(&[
        F::T("a"),
        F::Dot,
        F::T("b"),
        F::Dot,
        F::T("c"),
        F::Dot,
    ]);
    run(&[
        F::T("ab"),
        F::E("\"x"),
    ]);
    run(&[
        F::T("ab"),
        F::Dot,
        F::T("ab"),
        F::E("\"x"),
    ]);
    run(&[
        F::E("\"x . %"),
    ]);
    run(&[
        F::C("% \" . \""),
        F::T("\n"),
    ]);
    run(&[
        F::S("\"% . \""),
    ]);
    run(&[
        F::T("/ a"),
    ]);
    run(&[
        F::E("/* a"),
    ]);
    run(&[
        F::C("/* */"),
    ]);
    run(&[
        F::C("/*/**/*/"),
    ]);
    run(&[
        F::E("/*/**/ab"),
    ]);
}
