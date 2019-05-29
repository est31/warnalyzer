use defs::{CrateSaveAnalysis, CrateDisambiguator};
use StrErr;
use std::path::Path;
use std::collections::{HashSet, HashMap};

pub struct AnalysisDb {
	crates :HashMap<CrateDisambiguator, CrateSaveAnalysis>,
}

fn parse_save_analysis(path :&Path) -> Result<CrateSaveAnalysis, StrErr> {
	let file = std::fs::File::open(path)?;
	let file_parsed :CrateSaveAnalysis = serde_json::from_reader(file)?;
	Ok(file_parsed)
}

impl AnalysisDb {
	pub fn from_path(path :&str) -> Result<Self, StrErr> {
		let path = Path::new(path);
		let leaf_parsed = parse_save_analysis(&path)?;
		let disambiguators = leaf_parsed.prelude.external_crates.iter()
			.map(|v| v.id.disambiguator)
			.collect::<HashSet<_>>();
		let dir_path = path.parent().unwrap();
		let mut crates = HashMap::new();
		for entry in std::fs::read_dir(dir_path)? {
			let entry = entry?;
			let path = entry.path();
			let file_parsed = parse_save_analysis(&path)?;
			let disambiguator = file_parsed.prelude.crate_id.disambiguator;
			// Ignore results from prior compile runs
			if !disambiguators.contains(&disambiguator) {
				continue;
			}
			crates.insert(disambiguator, file_parsed);
		}
		println!("{:#?}", leaf_parsed);
		Ok(AnalysisDb {
			crates,
		})
	}
}
