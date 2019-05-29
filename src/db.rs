use defs::{CrateSaveAnalysis, CrateDisambiguator};
use StrErr;
use std::path::Path;
use std::collections::{HashSet, HashMap};

use defs::{Def, Ref, ItemId, Prelude};

pub type AbsItemId = ItemId<CrateDisambiguator>;

pub type AbsDef = Def<CrateDisambiguator>;
pub type AbsRef = Ref<CrateDisambiguator>;

pub struct AnalysisDb {
	crates :HashMap<CrateDisambiguator, CrateSaveAnalysis>,
	defs :HashMap<AbsItemId, AbsDef>,
	refs :HashMap<AbsItemId, AbsRef>,
}

impl<T> Def<T> {
	fn clone_map<U>(&self, f :impl FnOnce(&T) -> U) -> Def<U> {
		Def {
			kind : self.kind.clone(),
			name : self.name.clone(),
			id : ItemId {
				krate : f(&self.id.krate),
				index : self.id.index,
			},
			span : self.span.clone(),
		}
	}
}

impl<T> Ref<T> {
	fn clone_map<U>(&self, f :impl FnOnce(&T) -> U) -> Ref<U> {
		Ref {
			kind : self.kind.clone(),
			ref_id : ItemId {
				krate : f(&self.ref_id.krate),
				index : self.ref_id.index,
			},
			span : self.span.clone(),
		}
	}
}

fn parse_save_analysis(path :&Path) -> Result<CrateSaveAnalysis, StrErr> {
	let file = std::fs::File::open(path)?;
	let file_parsed :CrateSaveAnalysis = serde_json::from_reader(file)?;
	Ok(file_parsed)
}

impl Prelude {
	fn disambiguator_for_id(&self, id :u32) -> CrateDisambiguator {
		if id == 0 {
			return self.crate_id.disambiguator;
		}
		let krate = &self.external_crates[(id - 1) as usize];
		assert_eq!(krate.num, id);
		krate.id.disambiguator
	}
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
		let mut defs = HashMap::new();
		for (_dis, c) in crates.iter() {
			for v in c.defs.iter() {
				let v = v.clone_map(|w| c.prelude.disambiguator_for_id(*w));
				defs.insert(v.id, v);
			}
		}
		let mut refs = HashMap::new();
		for (_dis, c) in crates.iter() {
			for v in c.refs.iter() {
				let v = v.clone_map(|w| c.prelude.disambiguator_for_id(*w));
				refs.insert(v.ref_id, v);
			}
		}
		/*
		let defs = crates.iter()
			.map(|(_dis, c)| c.defs.iter().map(|v| (&c.prelude, v)))
			.flatten()
			.map(|(prelude, v)| {
				let v = v.clone_map(|w| prelude.disambiguator_for_id(*w));
				(v.id, v)
			})
			.collect::<HashMap<_, _>>();
		let refs = crates.iter()
			.map(|(_dis, c)| c.refs.iter().map(|v| (&c.prelude, v)))
			.flatten()
			.map(|(prelude, v)| {
				let v = v.clone_map(|w| prelude.disambiguator_for_id(*w));
				(v.ref_id, v)
			})
			.collect::<HashMap<_, _>>();
		*/

		println!("{:#?}", leaf_parsed);
		Ok(AnalysisDb {
			crates,
			defs,
			refs,
		})
	}
}
