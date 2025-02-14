use rmx::prelude::*;

use rmx::blake3;

use crate::input::Source;

#[salsa::tracked]
pub struct SourceHash<'db> {
    hash: [u8; 32],
}

#[salsa::tracked]
pub fn source_hash<'db>(
    db: &'db dyn crate::Db,
    source: Source,
) -> SourceHash<'db> {
    let hash = blake3::hash(source.text(db).as_bytes()).into();
    SourceHash::new(db, hash)
}
