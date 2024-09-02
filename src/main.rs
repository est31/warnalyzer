#[macro_use]
extern crate log;

use warnalyzer::save_analysis::db::AnalysisDb;
use warnalyzer::{Options, StrErr};

fn main() -> Result<(), StrErr> {
	pretty_env_logger::init();
	let path = std::env::args().nth(1).expect("please specify path");
	info!("{}", path);
	let options = Options {
		recurse : false,
	};
	let db = AnalysisDb::from_path(&path, options)?;
	for ud in db.get_unused_defs() {
		println!("{}: unused {} '{}'", ud.span.display_str(), ud.kind, ud.name);
	}

	Ok(())
}
