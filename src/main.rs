extern crate serde;
extern crate serde_json;

mod defs;

use defs::CrateSaveAnalysis;
use std::fmt::Display;

#[derive(Debug)]
pub struct StrErr(String);

impl<T :Display> From<T> for StrErr {
	fn from(v :T) -> Self {
		StrErr(format!("{}", v))
	}
}

fn main() -> Result<(), StrErr> {
	let file_name = std::env::args().nth(1).expect("please specify file name");
	println!("{}", file_name);
	let file = std::fs::File::open(file_name)?;
	let file_parsed :CrateSaveAnalysis = serde_json::from_reader(file)?;
	println!("{:?}", file_parsed);
	Ok(())
}
