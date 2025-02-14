use rmx::prelude::*;

use rmx::blake3;
use rmx::alloc::collections::{BTreeMap, BTreeSet};
use rmx::std::sync::Arc;

use crate::input::Source;

mod scratch {
    use super::*;

    #[salsa::tracked]
    pub fn translate_module<'db>(
        db: &'db dyn crate::Db,
        modmap: ModuleMap,
        module: Module,
    ) -> Translated<'db> {
        todo!()
    }

    #[salsa::tracked]
    pub struct Translated<'db> {
    }
}

#[salsa::input]
pub struct ModuleMap {
    #[return_ref]
    modules: BTreeSet<Module>,
}

#[salsa::input]
pub struct Module {
    pub source: Source,
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

