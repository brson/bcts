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
    pub source: Source,
}

#[salsa::input]
pub struct ModuleConfig {
    #[returns(ref)]
    pub import_config: ImportWorldConfig,
}

/// Defines the "world" of modules available for another module to import.
#[salsa::input]
pub struct ImportWorldConfig {
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
