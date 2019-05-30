extern crate serde;
extern crate serde_json;
extern crate syn;
extern crate proc_macro2;

mod defs;
mod db;

use std::fmt::Display;

#[derive(Debug)]
pub struct StrErr(String);

impl<T :Display> From<T> for StrErr {
	fn from(v :T) -> Self {
		StrErr(format!("{}", v))
	}
}

fn main() -> Result<(), StrErr> {
	let path = std::env::args().nth(1).expect("please specify path");
	println!("{}", path);
	let db = db::AnalysisDb::from_path(&path)?;
	for ud in db.get_unused_defs() {
		println!("{}: unused {} '{}'", ud.span.display_str(), ud.kind, ud.name);
	}

	Ok(())
}
