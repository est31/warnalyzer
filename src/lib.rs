#[macro_use]
extern crate log;

pub mod save_analysis;
pub mod scip;

use std::fmt::Display;

#[derive(Debug)]
pub struct StrErr(String);

impl<T :Display> From<T> for StrErr {
	fn from(v :T) -> Self {
		StrErr(format!("{}", v))
	}
}

#[derive(Clone)]
pub struct Options {
	pub recurse :bool,
}
