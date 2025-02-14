use rmx::prelude::*;

use rmx::blake3;
use rmx::alloc::collections::{BTreeMap, BTreeSet};
use rmx::std::sync::Arc;

use crate::input::Source;

#[salsa::input]
pub struct ModuleMap {
    #[return_ref]
    sources: BTreeMap<SourceHash, Source>,
    #[return_ref]
    modules: BTreeSet<Module>,
    #[return_ref]
    import_part_cache: BTreeMap<ImportPartStr, ImportPart>,
}

impl ModuleMap {
    pub fn empty(db: &dyn crate::Db) -> ModuleMap {
        ModuleMap::new(db, default(), default(), default())
    }

    pub fn insert_module(
        &mut self,
        db: &dyn crate::Db,
        module: Module,
    ) {
        todo!()
    }

    pub fn swap_modules(
        &mut self,
        db: &dyn crate::Db,
        modules: &[ModuleSwap],
    ) {
        todo!()
    }

    pub fn gc_import_part_cache(
        &mut self,
        db: &dyn crate::Db,
    ) {
        todo!()
    }
}

pub struct ModuleSwap {
    old: Module,
    new: Module,
}

#[salsa::input]
pub struct Module {
    pub source: SourceHash,
    pub config: ModuleConfig,
}

#[salsa::input]
pub struct ModuleConfig {
    #[return_ref]
    pub import_config: ImportConfig,
}

#[salsa::input]
pub struct ImportConfig {
    #[return_ref]
    pub imports: BTreeMap<ImportLocation, Module>,
}

#[salsa::input]
pub struct ImportLocation {
    #[return_ref]
    pub path: Vec<ImportPart>,
}

#[salsa::input]
pub struct ImportPart {
    #[return_ref]
    pub s: ImportPartStr,
}

pub type ImportPartStr = Arc<str>;

#[derive(Copy, Clone, Debug, salsa::Update)]
#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct SourceHash([u8; 32]);

impl Source {
    fn hash<'db>(&self, db: &'db dyn crate::Db) -> SourceHash {
        return source_hash(db, self.C()).hash(db);

        #[salsa::tracked]
        pub struct SourceHashTracked<'db> {
            hash: SourceHash,
        }

        #[salsa::tracked]
        pub fn source_hash<'db>(
            db: &'db dyn crate::Db,
            source: Source,
        ) -> SourceHashTracked<'db> {
            let hash = blake3::hash(source.text(db).as_bytes()).into();
            SourceHashTracked::new(db, SourceHash(hash))
        }
    }
}
