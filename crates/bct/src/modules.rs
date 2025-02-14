use rmx::prelude::*;

use rmx::blake3;
use rmx::alloc::collections::{BTreeMap, BTreeSet};
use rmx::std::sync::Arc;

use crate::input::Source;

#[salsa::input]
pub struct ModuleMap {
    #[return_ref]
    pub sources: BTreeMap<SourceHashId, Source>,
    #[return_ref]
    pub modules: BTreeSet<Module>,
    #[return_ref]
    pub import_part_cache: BTreeSet<ImportPart>,
}

#[salsa::input]
pub struct ModuleConfig {
    #[return_ref]
    pub import_configs: Vec<ImportConfig>,
}

#[salsa::input]
pub struct ImportConfig {
    #[return_ref]
    pub imports: BTreeMap<ImportPrefix, Module>,
}

pub type ImportPart = Arc<str>;

#[derive(Clone, Debug, salsa::Update)]
#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct ImportPrefix(pub Vec<ImportPart>);

#[salsa::input]
pub struct Module {
    source: SourceHashId,
    config: ModuleConfig,
}

#[derive(Copy, Clone, Debug, salsa::Update)]
#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct SourceHashId([u8; 32]);

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
