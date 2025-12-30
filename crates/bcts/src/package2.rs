use rmx::prelude::*;
use rmx::std::collections::BTreeMap;

use crate::input::Source;
use crate::package_resolve2::PackageWorldMap;

pub type PackageName = String;
pub type ModuleName = String;

#[salsa::input]
pub struct Package {
    #[returns(ref)]
    pub name: PackageName,
    #[returns(ref)]
    pub modules: BTreeMap<ModuleName, PackageModule>,
}

#[salsa::input]
#[derive(Ord, PartialOrd)]
pub struct PackageModule {
    #[returns(ref)]
    pub name: ModuleName,
    pub text: Source,
}

/// A package world containing system and local package libraries.
#[salsa::input]
pub struct PackageWorld {
    #[returns(ref)]
    pub pkglib_system: BTreeMap<PackageName, Package>,
    #[returns(ref)]
    pub pkglib_local: BTreeMap<PackageName, Package>,
}

/// Create a PackageWorldMap from a PackageWorld.
#[salsa::tracked]
pub fn package_world_map(
    db: &dyn salsa::Database,
    package_world: PackageWorld,
) -> PackageWorldMap<'_> {
    let pkglib_system = package_world.pkglib_system(db).C();
    let pkglib_local = package_world.pkglib_local(db).C();
    PackageWorldMap::new(
        db,
        BTreeMap::from([
            (S("sys"), pkglib_system),
            (S("local"), pkglib_local),
        ]),
    )
}

