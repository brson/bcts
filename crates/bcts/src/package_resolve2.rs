use rmx::prelude::*;
use rmx::std::collections::{BTreeSet, BTreeMap};
use rmx::std::path::PathBuf;

use crate::text::SubText;
use crate::package2::{self as package, PackageName, Package, PackageModule, ModuleName};

pub type ImportSpace = String;
pub type PackageAlias = String;
pub type ModuleAlias = String;

#[salsa::tracked]
pub struct PackageWorldMap<'db> {
    #[returns(ref)]
    pub map: BTreeMap<ImportSpace, BTreeMap<PackageName, Package>>,
}

pub type ImportDemand = (ImportSpace, PackageAlias, ModuleAlias);

#[salsa::tracked]
pub struct ImportDemandMap<'db> {
    #[returns(ref)]
    pub map: BTreeMap<PackageModule, Vec<ImportDemand>>,
}

#[salsa::tracked]
pub struct PackageWorldModuleGraph<'db> {
    #[returns(ref)]
    pub map: BTreeMap<PackageModule, BTreeSet<(ImportDemand, ResolvedPackageModule)>>,
}

#[derive(Copy, Clone, Hash, salsa::Update)]
#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub enum ResolvedPackageModule {
    Resolved(PackageModule),
    Unresolved,
}

#[salsa::tracked]
pub struct PackageWorldModuleGraphWithErrors<'db> {
    pub result: Result<PackageWorldModuleGraph<'db>, ValidationError>,
}

#[derive(Copy, Clone, Debug, Hash, salsa::Update)]
#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub enum ValidationError {
    CycleDetected,
}

#[salsa::tracked]
pub fn resolve_package_world<'db>(
    db: &'db dyn crate::Db,
    package_world_map: PackageWorldMap<'db>,
    import_demand_map: ImportDemandMap<'db>,
) -> PackageWorldModuleGraphWithErrors<'db> {
    let mut module_edges: BTreeMap<PackageModule, BTreeSet<(ImportDemand, ResolvedPackageModule)>> = default();
    for package_world_record in package_world_map.flatten_iter(db) {
        let PackageWorldRecord {
            import_space,
            package_name,
            package,
            package_module,
        } = package_world_record;
        let mut module_deps = BTreeSet::new();
        let import_demands = &import_demand_map.map(db)[&package_module];
        for import_demand in import_demands.iter() {
            let module_world_map = module_world_map(db, package_world_map, package);
            match lookup_import(
                db,
                module_world_map,
                import_demand,
            ) {
                Some(import_package_module) => {
                    module_deps.insert((
                        import_demand.C(), ResolvedPackageModule::Resolved(import_package_module),
                    ));
                },
                None => {
                    module_deps.insert((
                        import_demand.C(), ResolvedPackageModule::Unresolved,
                    ));
                }
            }
        }
        module_edges.insert(package_module, module_deps);
    }
    let graph = PackageWorldModuleGraph::new(db, module_edges);
    let result = validate_graph(db, graph).map(|()| graph);
    PackageWorldModuleGraphWithErrors::new(
        db,
        result,
    )
}

fn lookup_import<'db>(
    db: &'db dyn crate::Db,
    module_world_map: ModuleWorldMap,
    import_demand: &ImportDemand,
) -> Option<PackageModule> {
    let import_space = &import_demand.0;
    let package_alias = &import_demand.1;
    let module_alias = &import_demand.2;
    module_world_map.map(db).get(import_space)
        .and_then(|modules| {
            if import_space == "pkg" {
                modules.get(module_alias).copied()
            } else {
                let full_path = format!("{}/{}", package_alias, module_alias);
                modules.get(&full_path).copied()
            }
        })
}

fn validate_graph<'db>(
    db: &'db dyn crate::Db,
    graph: PackageWorldModuleGraph<'db>,
) -> Result<(), ValidationError> {
    let edges: BTreeMap<PackageModule, BTreeSet<PackageModule>> = graph.edges(db);
    detect_cycles(&edges)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

fn detect_cycles(edges: &BTreeMap<PackageModule, BTreeSet<PackageModule>>) -> Result<(), ValidationError> {
    let mut visit_state: BTreeMap<PackageModule, VisitState> = BTreeMap::new();

    // Initialize all nodes as unvisited
    for &node in edges.keys() {
        visit_state.insert(node, VisitState::Unvisited);
    }

    // All deps are accounted for
    for deps in edges.values() {
        for &dep in deps {
            assert!(visit_state.get(&dep).is_some());
            //visit_state.entry(dep).or_insert(VisitState::Unvisited);
        }
    }

    // Perform DFS from each unvisited node
    let nodes: Vec<PackageModule> = visit_state.keys().copied().collect();
    for node in nodes {
        if visit_state[&node] == VisitState::Unvisited {
            if dfs_detect_cycle(node, edges, &mut visit_state) {
                return Err(ValidationError::CycleDetected);
            }
        }
    }

    return Ok(());
}

fn dfs_detect_cycle(
    node: PackageModule,
    edges: &BTreeMap<PackageModule, BTreeSet<PackageModule>>,
    visit_state: &mut BTreeMap<PackageModule, VisitState>,
) -> bool {
    if visit_state[&node] == VisitState::Visiting {
        // Found a back edge - cycle detected
        return true;
    }

    if visit_state[&node] == VisitState::Visited {
        return false;
    }

    // Mark as visiting
    visit_state.insert(node, VisitState::Visiting);

    // Visit all dependencies
    if let Some(deps) = edges.get(&node) {
        for &dep in deps {
            if rmx::extras::recurse(|| {
                dfs_detect_cycle(dep, edges, visit_state)
            }) {
                return true;
            }
        }
    }

    // Mark as visited
    visit_state.insert(node, VisitState::Visited);

    false
}

#[salsa::tracked]
pub fn module_world_map<'db>(
    db: &'db dyn crate::Db,
    package_world_map: PackageWorldMap<'db>,
    package: Package,
) -> ModuleWorldMap<'db> {
    let mut module_map = package_world_map.module_map(db);
    assert!(module_map.get("pkg").is_none());
    module_map.insert(
        S("pkg"),
        package.modules(db).C(),
    );
    ModuleWorldMap::new(
        db,
        module_map,
    )
}

#[salsa::tracked]
struct ModuleWorldMap<'db> {
    #[returns(ref)]
    map: BTreeMap<ImportSpace, BTreeMap<ModuleAlias, PackageModule>>,
}

pub struct PackageWorldRecord<'db> {
    pub import_space: &'db str,
    pub package_name: &'db str,
    pub package: Package,
    pub package_module: PackageModule,
}

impl<'db> PackageWorldMap<'db> {
    fn module_map(
        &self,
        db: &'db dyn crate::Db,
    ) -> BTreeMap<ImportSpace, BTreeMap<ModuleName, PackageModule>> {
        self.map(db).iter()
            .map(|(import_space, packages)| -> (ImportSpace, BTreeMap<_, _>) {
                (
                    import_space.S(),
                    packages.iter().flat_map(|(package_name, package)| {
                        package.modules(db).iter().map(move |(module_name, package_module)| {
                            (
                                format!("{}/{}", package_name, module_name),
                                *package_module,
                            )
                        })
                    }).collect()
                )
            }).collect()
    }

    pub fn flatten_iter(
        &self,
        db: &'db dyn crate::Db,
    ) -> impl Iterator<Item = PackageWorldRecord<'db>> {
        let map = self.map(db);
        map.iter()
            .flat_map(move |(import_space, packages)| {
                packages.iter().flat_map(move |(package_name, package)| {
                    let modules = package.modules(db);
                    modules.iter().map(move |(module_name, package_module)| {
                        PackageWorldRecord {
                            import_space,
                            package_name,
                            package: *package,
                            package_module: *package_module,
                        }
                    })
                })
            })
    }
}

impl<'db> PackageWorldModuleGraph<'db> {
    fn edges(
        &self,
        db: &'db dyn crate::Db,
    ) -> BTreeMap<PackageModule, BTreeSet<PackageModule>> {
        self.map(db).iter().map(|(module, modules)| {
            let modules: BTreeSet<_> = modules.iter()
                .filter_map(|(_, module)| match module {
                    ResolvedPackageModule::Resolved(module) => Some(module),
                    ResolvedPackageModule::Unresolved => None,
                }).copied().collect();
            (*module, modules)
        }).collect()
    }
}

#[cfg(test)]
use crate::input::Source;

#[cfg(test)]
#[rustfmt::skip]
#[salsa::tracked]
fn test_map<'db>(
    db: &'db dyn crate::Db,
) -> PackageWorldMap<'db> {
    PackageWorldMap::new(
        db,
        BTreeMap::from([
            (S("main"), BTreeMap::from([
                (S("main"), Package::new(
                    db,
                    S("main"),
                    BTreeMap::from([
                        (S("main"), PackageModule::new(
                            db,
                            S("main"),
                            Source::new(
                                db,
                                S("import module sys/core"),
                            ),
                        )),
                    ]),
                )),
            ])),
            (S("sys"), BTreeMap::from([
                (S("core"), Package::new(
                    db,
                    S("core"),
                    BTreeMap::from([
                        (S("core"), PackageModule::new(
                            db,
                            S("core"),
                            Source::new(
                                db,
                                S("import module pkg/u32"),
                            ),
                        )),
                        (S("u32"), PackageModule::new(
                            db,
                            S("u32"),
                            Source::new(
                                db,
                                S(""),
                            ),
                        )),
                    ]),
                )),
                (S("alloc"), Package::new(
                    db,
                    S("alloc"),
                    BTreeMap::from([
                        (S("alloc"), PackageModule::new(
                            db,
                            S("alloc"),
                            Source::new(
                                db,
                                S("import module sys/core"),
                            ),
                        )),
                    ]),
                )),
            ])),
        ]),
    )
}

#[test]
fn package_world_map_iter_lazy() {
    let ref db = crate::Database::default();
    let map = test_map(db);
    let expected = vec![
        ("main", "main", "main"),
        ("sys", "alloc", "alloc"),
        ("sys", "core", "core"),
        ("sys", "core", "u32"),
    ];
    let actual: Vec<_> = map.flatten_iter(db).collect();
    assert_eq!(expected.len(), actual.len());
    for (expected, actual) in expected.into_iter().zip(actual.into_iter()) {
        assert_eq!(expected.0, actual.import_space);
        assert_eq!(expected.1, actual.package_name);
        assert_eq!(expected.1, actual.package.name(db));
        assert_eq!(expected.2, actual.package_module.name(db));
    }
}

#[salsa::tracked]
struct TestInput<'db> {
    package_world_map: PackageWorldMap<'db>,
    import_demand_map: ImportDemandMap<'db>,
}

#[cfg(test)]
#[rustfmt::skip]
#[salsa::tracked]
fn test_input_unresolvable<'db>(
    db: &'db dyn crate::Db,
) -> TestInput<'db> {
    let package_world_map = PackageWorldMap::new(
        db,
        BTreeMap::from([
            (S("main"), BTreeMap::from([
                (S("main"), Package::new(
                    db,
                    S("main"),
                    BTreeMap::from([
                        (S("main"), PackageModule::new(
                            db,
                            S("main"),
                            Source::new(
                                db,
                                S("import module sys/core"),
                            ),
                        )),
                    ]),
                )),
            ])),
        ]),
    );
    let module_map = package_world_map.module_map(db);
    let import_demand_map = ImportDemandMap::new(
        db,
        BTreeMap::from([
            (module_map["main"]["main/main"], vec![
                (S("sys"), S("core"), S("core"))
            ]),
        ]),
    );
    TestInput::new(db, package_world_map, import_demand_map)
}

#[test]
fn test_unresolved_import() {
    let ref db = crate::Database::default();
    let test_input = test_input_unresolvable(db);
    let resolved = resolve_package_world(
        db,
        test_input.package_world_map(db),
        test_input.import_demand_map(db),
    );
    let unresolved_expected = vec![
        ("main", ("sys", "core", "core")),
    ];

    let mut unresolved_actual = vec![];
    for (package_module, imports) in resolved.result(db).expect(".").map(db) {
        for ((import_space, package_alias, module_alias), resolved_package) in imports {
            if matches!(resolved_package, ResolvedPackageModule::Unresolved) {
                unresolved_actual.push(
                    (
                        package_module.name(db).as_str(),
                        (import_space.as_str(), package_alias.as_str(), module_alias.as_str())
                    )
                );
            }
        }
    }

    assert_eq!(unresolved_expected.len(), unresolved_actual.len());
    for (expected, actual) in unresolved_expected.into_iter().zip(unresolved_actual.into_iter()) {
        assert_eq!(expected, actual);
    }
}

#[cfg(test)]
#[rustfmt::skip]
#[salsa::tracked]
fn test_input_cycle<'db>(
    db: &'db dyn crate::Db,
) -> TestInput<'db> {
    let package_world_map = PackageWorldMap::new(
        db,
        BTreeMap::from([
            (S("sys"), BTreeMap::from([
                (S("core"), Package::new(
                    db,
                    S("core"),
                    BTreeMap::from([
                        (S("core"), PackageModule::new(
                            db,
                            S("core"),
                            Source::new(
                                db,
                                S("import module pkg/b"),
                            ),
                        )),
                        (S("a"), PackageModule::new(
                            db,
                            S("a"),
                            Source::new(
                                db,
                                S("import module pkg/b"),
                            ),
                        )),
                        (S("b"), PackageModule::new(
                            db,
                            S("b"),
                            Source::new(
                                db,
                                S("import module pkg/a"),
                            ),
                        )),
                    ]),
                )),
            ])),
        ]),
    );
    let package_core = package_world_map.map(db)["sys"]["core"];
    let module_core = package_core.modules(db)["core"];
    let module_a = package_core.modules(db)["a"];
    let module_b = package_core.modules(db)["b"];
    let import_demand_map = ImportDemandMap::new(
        db,
        BTreeMap::from([
            (module_core, vec![
                (S("pkg"), S("core"), S("b"))
            ]),
            (module_a, vec![
                (S("pkg"), S("core"), S("b"))
            ]),
            (module_b, vec![
                (S("pkg"), S("core"), S("a"))
            ]),
        ]),
    );
    TestInput::new(db, package_world_map, import_demand_map)
}

#[test]
fn test_cycles() {
    let ref db = crate::Database::default();
    let test_input = test_input_cycle(db);
    let resolved = resolve_package_world(
        db,
        test_input.package_world_map(db),
        test_input.import_demand_map(db),
    );

    assert!(resolved.result(db).is_err());
}
