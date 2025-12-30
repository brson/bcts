//! Module graph abstraction.
//!
//! Provides a package-agnostic view of modules for compilation.
//! `ModuleGraph` represents a dependency-ordered collection of modules.

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

/// The module graph: a dependency-ordered collection of modules.
///
/// Contains all modules in topological order (dependencies before dependents).
/// Function-level imports are resolved by the typechecker, not stored here.
#[salsa::input]
pub struct ModuleGraph {
    /// Modules in dependency order (dependencies come first).
    #[returns(ref)]
    pub modules: Vec<Module>,

    /// Module lookup by ID.
    #[returns(ref)]
    pub module_by_id: BTreeMap<ModuleId, Module>,

    /// Direct dependencies per module (for ordering verification).
    #[returns(ref)]
    pub dependencies: BTreeMap<ModuleId, BTreeSet<ModuleId>>,
}

impl ModuleGraph {
    /// Get a module by its ID.
    pub fn get_module(&self, db: &dyn salsa::Database, id: ModuleId) -> Option<Module> {
        self.module_by_id(db).get(&id).copied()
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
    dependencies: BTreeMap<ModuleId, BTreeSet<ModuleId>>,
}

impl<'db> ModuleGraphBuilder<'db> {
    /// Create a new builder.
    pub fn new(db: &'db dyn salsa::Database) -> Self {
        Self {
            db,
            modules: Vec::new(),
            module_by_id: BTreeMap::new(),
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
        self.dependencies.insert(id, BTreeSet::new());
        id
    }

    /// Add a dependency between modules.
    pub fn add_dependency(&mut self, module_id: ModuleId, depends_on: ModuleId) {
        if let Some(deps) = self.dependencies.get_mut(&module_id) {
            deps.insert(depends_on);
        }
    }

    /// Build the final ModuleGraph.
    pub fn build(self) -> ModuleGraph {
        ModuleGraph::new(
            self.db,
            self.modules,
            self.module_by_id,
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

        // math depends on base.
        builder.add_dependency(math, base);

        let graph = builder.build();

        // Verify structure.
        assert_eq!(graph.modules(&db).len(), 2);
        assert!(graph.get_module(&db, base).is_some());
        assert!(graph.get_module(&db, math).is_some());

        // Verify dependencies.
        let math_deps = graph.dependencies(&db).get(&math).unwrap();
        assert!(math_deps.contains(&base));
    }
}
