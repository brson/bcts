#![allow(unused)]
#![allow(clippy::needless_lifetimes)]

use rmx::prelude::*;

pub mod input;
pub mod text;
pub mod chunk;
pub mod source_map;
pub mod chunks;
pub mod lexer;
pub mod bracer;
pub mod lines;

pub mod modules;
pub mod import_resolve;
//pub mod package;

use salsa::Database as Db;

#[salsa::db]
#[derive(Default, Clone)]
pub struct Database {
    storage: salsa::Storage<Self>,
}

#[salsa::db]
impl salsa::Database for Database {
    fn salsa_event(&self, event: &dyn Fn() -> salsa::Event) {
        let event = event();
        info!("event: {event:?}");
    }
}
