use rmx::prelude::*;
use rmx::std::collections::BTreeMap;

use crate::input::Source;

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

