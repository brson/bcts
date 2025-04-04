use rmx::prelude::*;

use crate::modules::{
    Module,
    ModuleMap,
    ImportLocation,
};

#[salsa::tracked]
pub struct Imports<'db> {
    pub imports: Vec<ImportLocation>,
}

#[salsa::tracked]
pub fn resolve_imports<'db>(
    db: &'db dyn crate::Db,
    module_map: ModuleMap,
    module: Module,
    imports: Imports<'db>,
) -> ResolvedImports<'db> {
    let module_config = module_map.configs(db).get(&module)
        .expect("module_map.config");
    let available_modules = module_config.import_config(db).modules(db);
    let resolved = imports.imports(db).iter().map(|loc| {
        available_modules.get(loc).cloned().ok_or_else(|| ())
    }).collect();
    ResolvedImports::new(db, resolved)
}

#[salsa::tracked]
pub struct ResolvedImports<'db> {
    pub imports: Vec<Result<Module, ()>>,
}
