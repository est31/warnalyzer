#[macro_use]
extern crate log;

use warnalyzer::{Options, StrErr};

fn main() -> Result<(), StrErr> {
	pretty_env_logger::init();
	let path = std::env::args().nth(1).expect("please specify path");
	info!("{}", path);
	let options = Options {
		recurse : false,
	};
	let is_json = path.ends_with(".json");
	let is_scip = path.ends_with(".scip");
	if is_json {
		let db = warnalyzer::save_analysis::db::AnalysisDb::from_path(&path, options)?;
		for ud in db.get_unused_defs() {
			println!("{}: unused {} '{}'", ud.span.display_str(), ud.kind, ud.name);
		}
	} else if is_scip {
		let db = warnalyzer::scip::AnalysisDb::from_path(&path, options)?;
		db.dump_index()?;
		for ud in db.get_unused_defs() {
			println!("{}: unused {:?} '{:?}'", ud.span.display_str(), ud.kind, ud.name);
		}
	} else {
		eprintln!("Path '{path}' has unknown extension");
	}

	Ok(())
}
