use rmx::prelude::*;

use rmx::std::ops::Range;
use rmx::std::{iter, mem};
use rmx::std::iter::Peekable;
use rmx::std::slice::Iter as SliceIter;

use crate::text::Text;

#[salsa::tracked]
pub struct Chunk<'db> {
    pub text: Text<'db>,
    #[returns(ref)]
    pub comments: Vec<Range<usize>>,
    #[returns(ref)]
    pub strings: Vec<Range<usize>>,
    #[returns(ref)]
    pub errors: Vec<Range<usize>>,
}

impl<'db> Chunk<'db> {
    pub fn ranges(
        &self,
        db: &'db dyn crate::Db,
    ) -> impl Iterator<Item = (Range<usize>, RangeKind)> + use<'db> {
        let comments = self.comments(db).iter().cloned().map(|range| (range, RangeKind::Comment));
        let strings = self.strings(db).iter().cloned().map(|range| (range, RangeKind::String));
        let errors = self.errors(db).iter().cloned().map(|range| (range, RangeKind::Error));
        let mut known_ranges = comments
            .merge_by(strings, |x, y| x.0.start <= y.0.start)
            .merge_by(errors, |x, y| x.0.start <= y.0.start);

        let next_known_range = known_ranges.next();
        let mut ranges = Ranges {
            chunk_len: self.text(db).as_str(db).len(),
            known_ranges: Box::new(known_ranges),
            next_known_range,
            position: 0,
        };
        iter::from_fn(move || ranges.next())
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum RangeKind { Comment, String, Error, Unknown }

struct Ranges<'db> {
    chunk_len: usize,
    known_ranges: Box<dyn Iterator<Item = (Range<usize>, RangeKind)> + 'db>,
    next_known_range: Option<(Range<usize>, RangeKind)>,
    position: usize,
}

impl<'db> Ranges<'db> {
    fn next(&mut self) -> Option<(Range<usize>, RangeKind)> {
        match self.next_known_range.C() {
            None => {
                if self.position < self.chunk_len {
                    let range = self.position .. self.chunk_len;
                    self.position = range.end;
                    Some((range, RangeKind::Unknown))
                } else {
                    None
                }
            }
            Some((known_range, kind)) => {
                match self.position.cmp(&known_range.start) {
                    Ordering::Less => {
                        let range = self.position .. known_range.start;
                        self.position = known_range.start;
                        Some((range, RangeKind::Unknown))
                    }
                    Ordering::Equal => {
                        self.position = known_range.end;
                        self.next_known_range = self.known_ranges.next();
                        Some((known_range, kind))
                    }
                    Ordering::Greater => unreachable!(),
                }
            }
        }
    }
}

