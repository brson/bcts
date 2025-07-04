use rmx::prelude::*;

use std::ops::Range;
use std::{iter, mem};

#[salsa::tracked]
pub struct Text<'db> {
    #[returns(ref)]
    pub text: String,
}

#[salsa::tracked]
pub struct SubText<'db> {
    pub text: Text<'db>,
    pub range: Range<usize>,
}

#[salsa::interned]
#[derive(Debug)]
pub struct InternedText<'db> {
    #[returns(ref)]
    pub text: String,
}

#[salsa::interned]
#[derive(Debug)]
pub struct InternedSubText<'db> {
    pub text: InternedText<'db>,
    pub range: Range<usize>,
}

impl<'db> Text<'db> {
    pub fn as_sub(
        &self,
        db: &'db dyn crate::Db,
    ) -> SubText<'db> {
        SubText::new(
            db,
            *self,
            (0..self.text(db).len()),
        )
    }

    pub fn sub(
        &self,
        db: &'db dyn crate::Db,
        range: Range<usize>,
    ) -> SubText<'db> {
        SubText::new(
            db,
            *self,
            range,
        )
    }
}

impl<'db> SubText<'db> {
    pub fn sub(
        &self,
        db: &'db dyn crate::Db,
        range: Range<usize>,
    ) -> Option<SubText<'db>> {
        let text_len = self.text(db).text(db).len();
        let subrange = (0..text_len).subrange(range)?;
        Some(SubText::new(
            db,
            self.text(db),
            subrange,
        ))
    }
}

impl<'db> InternedText<'db> {
    pub fn as_sub(
        &self,
        db: &'db dyn crate::Db,
    ) -> InternedSubText<'db> {
        InternedSubText::new(
            db,
            *self,
            (0..self.text(db).len()),
        )
    }

    pub fn sub(
        &self,
        db: &'db dyn crate::Db,
        range: Range<usize>,
    ) -> InternedSubText<'db> {
        InternedSubText::new(
            db,
            *self,
            range,
        )
    }
}

impl<'db> InternedSubText<'db> {
    pub fn sub(
        &self,
        db: &'db dyn crate::Db,
        range: Range<usize>,
    ) -> Option<InternedSubText<'db>> {
        let text_len = self.text(db).text(db).len();
        let subrange = (0..text_len).subrange(range)?;
        Some(InternedSubText::new(
            db,
            self.text(db),
            subrange,
        ))
    }
}

impl<'db> Text<'db> {
    pub fn as_str(&self, db: &'db dyn crate::Db) -> &'db str {
        self.text(db).as_str()
    }
}

impl<'db> SubText<'db> {
    pub fn as_str(&self, db: &'db dyn crate::Db) -> &'db str {
        &self.text(db).text(db)[self.range(db)]
    }
}

impl<'db> InternedText<'db> {
    pub fn as_str(&self, db: &'db dyn crate::Db) -> &'db str {
        self.text(db).as_str()
    }
}

impl<'db> InternedSubText<'db> {
    pub fn as_str(&self, db: &'db dyn crate::Db) -> &'db str {
        &self.text(db).text(db)[self.range(db)]
    }
}

#[test]
fn interned_text_eq() {
    let ref db = crate::Database::default();
    let s1 = InternedText::new(db, S("abc"));
    let s2 = InternedText::new(db, S("abc"));
    assert_eq!(s1, s2);

    #[salsa::tracked]
    fn test_sub<'db>(
        db: &'db dyn crate::Db,
        s1: InternedText<'db>,
        s2: InternedText<'db>,
    ) {
        let s1 = s1.sub(db, 1..2);
        let s2 = s2.sub(db, 1..2);
        assert_eq!(s1, s2);
    }

    test_sub(db, s1, s2);
}
