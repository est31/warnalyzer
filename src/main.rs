#[macro_use]
extern crate log;

use std::fs::create_dir;

use warnalyzer::{scip::run_scip, Options, StrErr};

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
		report_scip(&path, options)?;
	} else {
		let path = std::path::Path::new(&path);
		if path.is_dir() {
			let target_dir = path.join("target");
			if !target_dir.exists() {
				create_dir(&target_dir)?;
			}
			let index_path = target_dir.join("index.scip");
			run_scip(&path, &index_path)?;
			report_scip(index_path.to_str().unwrap(), options)?;
		} else {
			eprintln!("Path '{}' doesn't exist or has unknown extension", path.display());
		}
	}

	Ok(())
}

fn report_scip(path: &str, options: Options) -> Result<(), StrErr> {
	let db = warnalyzer::scip::AnalysisDb::from_path(&path, options)?;
	//db.dump_index()?;
	for ud in db.get_unused_defs() {
		let kind = ud.kind.map(|s| format!("{s:?}")).unwrap_or_else(|| "<unknown>".to_owned());
		let name = ud.name.unwrap_or_default();
		println!("{}: unused {} '{}'", ud.span.display_str(), kind, name);
	}
	Ok(())
}