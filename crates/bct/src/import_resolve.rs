use rmx::prelude::*;

use crate::modules::{
    Module,
    ImportLocation,
};

#[salsa::tracked]
pub struct Imports<'db> {
    pub imports: Vec<ImportLocation>,
}

#[salsa::tracked]
pub fn resolve_imports<'db>(
    db: &'db dyn crate::Db,
    module: Module,
    imports: Imports<'db>,
) -> ResolvedImports<'db> {
    let available_modules = module.config(db).import_config(db).modules(db);
    let resolved = imports.imports(db).iter().map(|loc| {
        available_modules.get(loc).cloned().ok_or_else(|| ())
    }).collect();
    ResolvedImports::new(db, resolved)
}

#[salsa::tracked]
pub struct ResolvedImports<'db> {
    pub imports: Vec<Result<Module, ()>>,
}
