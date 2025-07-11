use rmx::prelude::*;
use rmx::std::collections::{BTreeSet, BTreeMap};
use rmx::std::path::PathBuf;

use crate::text::SubText;
use crate::package::{self, PackageName, Package, PackageModule, ModuleName};

pub type ImportSpace = String;
pub type ModuleAlias = String;

#[salsa::tracked]
pub struct PackageWorldMap<'db> {
    #[returns(ref)]
    pub map: BTreeMap<ImportSpace, BTreeMap<PackageName, Package>>,
}

pub type ImportDemand = (ImportSpace, ModuleAlias);

#[salsa::tracked]
pub struct ImportDemandMap<'db> {
    #[returns(ref)]
    pub map: BTreeMap<PackageModule, Vec<ImportDemand>>,
}

#[salsa::tracked]
pub struct PackageWorldModuleGraph<'db> {
    #[returns(ref)]
    pub map: BTreeMap<PackageModule, BTreeSet<(ImportDemand, PackageModule)>>,
}

#[salsa::tracked]
pub fn resolve_package_world<'db>(
    db: &'db dyn crate::Db,
    package_world_map: PackageWorldMap<'db>,
    import_demand_map: ImportDemandMap<'db>,
) -> PackageWorldModuleGraph<'db> {
    let mut module_edges: BTreeMap<PackageModule, BTreeSet<(ImportDemand, PackageModule)>> = default();
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
                    module_deps.insert((import_demand.C(), import_package_module));
                },
                None => todo!("unresolved module"),
            }
        }
        module_edges.insert(package_module, module_deps);
    }
    let graph = PackageWorldModuleGraph::new(db, module_edges);
    validate_graph(db, graph);
    graph
}

fn lookup_import<'db>(
    db: &'db dyn crate::Db,
    module_world_map: ModuleWorldMap,
    import_demand: &ImportDemand,
) -> Option<PackageModule> {
    let import_space = &import_demand.0;
    let module_alias = &import_demand.1;
    module_world_map.map(db).get(import_space)
        .map(|modules| {
            modules.get(module_alias).copied()
        }).flatten()
}

fn validate_graph<'db>(
    db: &'db dyn crate::Db,
    graph: PackageWorldModuleGraph<'db>,
) {
    todo!()
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

struct PackageWorldRecord<'db> {
    import_space: &'db str,
    package_name: &'db str,
    package: Package,
    package_module: PackageModule,
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
                    packages.iter().map(|(package_name, package)| {
                        (
                            package_name.S(),
                            package.modules(db)[package.main_module(db)],
                        )
                    }).collect()
                )
            }).collect()
    }

    fn flatten_iter(
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
