//! Module graph abstraction.
//!
//! Provides a package-agnostic view of modules for compilation.
//! `ModuleGraph` represents a dependency-ordered collection of modules
//! with resolved imports.

use rmx::prelude::*;
use rmx::std::collections::{BTreeMap, BTreeSet};
use crate::input::Source;

/// Opaque module identifier.
///
/// Modules are identified by their path string (e.g., "sys/std/u32").
#[salsa::input]
#[derive(Ord, PartialOrd)]
pub struct ModuleId {
    /// Module path (e.g., "sys/std/u32").
    #[returns(ref)]
    pub path: String,
}

/// A module in the graph.
#[salsa::input]
pub struct Module {
    /// Module identifier.
    pub id: ModuleId,
    /// Module source text.
    pub source: Source,
}

/// Resolved import: local alias maps to source module and export name.
#[derive(Clone, Hash, PartialEq, Eq)]
#[derive(salsa::Update)]
pub struct ResolvedImport {
    /// Local name used in this module.
    pub local_name: String,
    /// Source module ID.
    pub source_module: ModuleId,
    /// Name of the export in the source module.
    pub export_name: String,
}

impl std::fmt::Debug for ResolvedImport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedImport")
            .field("local_name", &self.local_name)
            .field("source_module", &"<ModuleId>")
            .field("export_name", &self.export_name)
            .finish()
    }
}

/// The module graph: a dependency-ordered collection of modules.
///
/// Contains all modules in topological order (dependencies before dependents)
/// and resolved imports for each module.
#[salsa::input]
pub struct ModuleGraph {
    /// Modules in dependency order (dependencies come first).
    #[returns(ref)]
    pub modules: Vec<Module>,

    /// Module lookup by ID.
    #[returns(ref)]
    pub module_by_id: BTreeMap<ModuleId, Module>,

    /// Resolved imports per module.
    #[returns(ref)]
    pub imports: BTreeMap<ModuleId, Vec<ResolvedImport>>,

    /// Direct dependencies per module (for ordering verification).
    #[returns(ref)]
    pub dependencies: BTreeMap<ModuleId, BTreeSet<ModuleId>>,
}

impl ModuleGraph {
    /// Get a module by its ID.
    pub fn get_module(&self, db: &dyn salsa::Database, id: ModuleId) -> Option<Module> {
        self.module_by_id(db).get(&id).copied()
    }

    /// Get imports for a module.
    pub fn get_imports<'db>(&self, db: &'db dyn salsa::Database, id: ModuleId) -> &'db [ResolvedImport] {
        self.imports(db).get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Iterate modules in dependency order.
    pub fn iter_modules<'db>(&self, db: &'db dyn salsa::Database) -> impl Iterator<Item = Module> + 'db {
        self.modules(db).iter().copied()
    }
}

/// Builder for constructing a ModuleGraph.
pub struct ModuleGraphBuilder<'db> {
    db: &'db dyn salsa::Database,
    modules: Vec<Module>,
    module_by_id: BTreeMap<ModuleId, Module>,
    imports: BTreeMap<ModuleId, Vec<ResolvedImport>>,
    dependencies: BTreeMap<ModuleId, BTreeSet<ModuleId>>,
}

impl<'db> ModuleGraphBuilder<'db> {
    /// Create a new builder.
    pub fn new(db: &'db dyn salsa::Database) -> Self {
        Self {
            db,
            modules: Vec::new(),
            module_by_id: BTreeMap::new(),
            imports: BTreeMap::new(),
            dependencies: BTreeMap::new(),
        }
    }

    /// Add a module to the graph.
    ///
    /// Modules must be added in dependency order (dependencies first).
    pub fn add_module(
        &mut self,
        path: impl Into<String>,
        source: Source,
    ) -> ModuleId {
        let id = ModuleId::new(self.db, path.into());
        let module = Module::new(self.db, id, source);
        self.modules.push(module);
        self.module_by_id.insert(id, module);
        self.imports.insert(id, Vec::new());
        self.dependencies.insert(id, BTreeSet::new());
        id
    }

    /// Add an import to a module.
    pub fn add_import(
        &mut self,
        module_id: ModuleId,
        local_name: impl Into<String>,
        source_module: ModuleId,
        export_name: impl Into<String>,
    ) {
        let import = ResolvedImport {
            local_name: local_name.into(),
            source_module,
            export_name: export_name.into(),
        };
        if let Some(imports) = self.imports.get_mut(&module_id) {
            imports.push(import);
        }
        if let Some(deps) = self.dependencies.get_mut(&module_id) {
            deps.insert(source_module);
        }
    }

    /// Build the final ModuleGraph.
    pub fn build(self) -> ModuleGraph {
        ModuleGraph::new(
            self.db,
            self.modules,
            self.module_by_id,
            self.imports,
            self.dependencies,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_graph_builder() {
        let db = crate::Database::default();
        let mut builder = ModuleGraphBuilder::new(&db);

        // Add modules in dependency order.
        let base = builder.add_module("sys/std/base", Source::new(&db, S("// base")));
        let math = builder.add_module("sys/std/math", Source::new(&db, S("// math")));

        // math imports from base.
        builder.add_import(math, "base_fn", base, "base_fn");

        let graph = builder.build();

        // Verify structure.
        assert_eq!(graph.modules(&db).len(), 2);
        assert!(graph.get_module(&db, base).is_some());
        assert!(graph.get_module(&db, math).is_some());

        let math_imports = graph.get_imports(&db, math);
        assert_eq!(math_imports.len(), 1);
        assert_eq!(math_imports[0].local_name, "base_fn");
    }
}
