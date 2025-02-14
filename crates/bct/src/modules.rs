use rmx::prelude::*;

use rmx::blake3;
use rmx::alloc::collections::BTreeMap;

use crate::input::Source;

#[derive(Copy, Clone, Debug, salsa::Update)]
#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct SourceHashId([u8; 32]);

#[salsa::input]
pub struct ModuleMap {
    #[return_ref]
    pub import_to_hash: BTreeMap<String, SourceHashId>,
}

#[salsa::tracked]
pub struct SourceHash<'db> {
    hash: SourceHashId,
}

#[salsa::tracked]
pub fn source_hash<'db>(
    db: &'db dyn crate::Db,
    source: Source,
) -> SourceHash<'db> {
    let hash = blake3::hash(source.text(db).as_bytes()).into();
    SourceHash::new(db, SourceHashId(hash))
}
