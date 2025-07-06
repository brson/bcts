use rmx::prelude::*;

use rmx::blake3;
use rmx::alloc::collections::{BTreeMap, BTreeSet};
use rmx::std::sync::Arc;

use crate::input::Source;

#[salsa::input]
pub struct ModuleMap {
    #[returns(ref)]
    modules: BTreeSet<Module>,
    #[returns(ref)]
    pub configs: BTreeMap<Module, ModuleConfig>,
}

#[salsa::input]
#[derive(Ord, PartialOrd)]
pub struct Module {
    #[returns(ref)]
    pub source: Source,
}

#[salsa::input]
pub struct ModuleConfig {
    #[returns(ref)]
    pub import_config: ImportConfig,
}

#[salsa::input]
pub struct ImportConfig {
    #[returns(ref)]
    pub modules: BTreeMap<ImportLocation, Module>,
}

#[salsa::input]
#[derive(Ord, PartialOrd)]
pub struct ImportLocation {
    #[returns(ref)]
    pub path: Vec<ImportPart>,
}

#[salsa::input]
pub struct ImportPart {
    #[returns(ref)]
    pub s: Arc<str>,
}
