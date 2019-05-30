extern crate serde;
extern crate serde_json;
extern crate syn;
extern crate proc_macro2;
extern crate intervaltree;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;
extern crate rayon;
extern crate chashmap;

mod defs;
mod db;
mod mute;

use std::fmt::Display;

#[derive(Debug)]
pub struct StrErr(String);

impl<T :Display> From<T> for StrErr {
	fn from(v :T) -> Self {
		StrErr(format!("{}", v))
	}
}

fn main() -> Result<(), StrErr> {
	pretty_env_logger::init();
	let path = std::env::args().nth(1).expect("please specify path");
	info!("{}", path);
	let db = db::AnalysisDb::from_path(&path)?;
	for ud in db.get_unused_defs() {
		println!("{}: unused {} '{}'", ud.span.display_str(), ud.kind, ud.name);
	}

	Ok(())
}
