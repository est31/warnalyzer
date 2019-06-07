use defs::{CrateSaveAnalysis, CrateDisambiguator,
	CrateSaveAnalysisMetadata};
use StrErr;
use std::path::{Path, PathBuf};
use std::collections::{HashSet, HashMap};
use rayon::prelude::*;

use mute::MuteSpansCache;

use defs::{Def, Ref, ItemId, Prelude};

pub type AbsItemId = ItemId<CrateDisambiguator>;

pub type AbsDef = Def<CrateDisambiguator>;
pub type AbsRef = Ref<CrateDisambiguator>;

pub struct AnalysisDb {
	root :Option<PathBuf>,
	covered_crates :HashSet<CrateDisambiguator>,
	defs :HashMap<AbsItemId, AbsDef>,
	refs :HashMap<AbsItemId, AbsRef>,
}

impl<T> ItemId<T> {
	fn clone_map<U>(&self, f :impl FnOnce(&T) -> U) -> ItemId<U> {
		ItemId {
			krate : f(&self.krate),
			index : self.index,
		}
	}
}

impl<T> Def<T> {
	fn clone_map<U>(&self, f :impl Fn(&T) -> U) -> Def<U> {
		Def {
			kind : self.kind.clone(),
			name : self.name.clone(),
			id : self.id.clone_map(&f),
			span : self.span.clone(),
			parent : self.parent.as_ref().map(|v| v.clone_map(&f)),
			decl_id : self.decl_id.as_ref().map(|v| v.clone_map(&f)),
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
	let file = std::fs::read_to_string(path)?;
	let file_parsed :CrateSaveAnalysis = serde_json::from_str(&file)?;
	Ok(file_parsed)
}
fn parse_analysis_metadata(path :&Path) -> Result<CrateSaveAnalysisMetadata, StrErr> {
	let file = std::fs::read_to_string(path)?;
	let file_parsed :CrateSaveAnalysisMetadata = serde_json::from_str(&file)?;
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
		let leaf_parsed = parse_analysis_metadata(&path)?;
		let mut disambiguators = leaf_parsed.prelude.external_crates.iter()
			.map(|v| v.id.disambiguator)
			.collect::<HashSet<_>>();
		disambiguators.insert(leaf_parsed.prelude.crate_id.disambiguator);
		let dir_path = path.parent().unwrap();
		let mut crates = HashMap::new();
		let mut covered_crates = HashSet::new();
		let v :Vec<_> = std::fs::read_dir(dir_path)?
			.collect::<Vec<_>>()
			.into_par_iter().map(|entry| -> Result<_, StrErr> {
				let entry = entry?;
				let path = entry.path();
				let metadata = parse_analysis_metadata(&path)?;
				let disambiguator = metadata.prelude.crate_id.disambiguator;
				// Ignore results from other compile runs
				if !disambiguators.contains(&disambiguator) {
					return Ok(None);
				}

				// Ignore stuff from crates.io or git deps.
				// Just focus on path deps for now.
				if metadata.compilation.directory.contains(".cargo/registry/src/github.com") ||
						metadata.compilation.directory.contains(".cargo/git/") {
					info!("i> {}", path.to_str().unwrap());
					return Ok(None);
				}
				info!("p> {}", path.to_str().unwrap());
				let file_parsed = parse_save_analysis(&path)?;
				Ok(Some((disambiguator, file_parsed)))
		}).collect();
		for v in v.into_iter() {
			let v = v?;
			if let Some((disambiguator, file_parsed)) = v {
				covered_crates.insert(disambiguator);
				crates.insert(disambiguator, file_parsed);
			}
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
		//println!("{:#?}", defs);
		//println!("{:#?}", refs);

		let root = path.parent()
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.map(|p| p.to_owned());
		Ok(AnalysisDb {
			root,
			covered_crates,
			defs,
			refs,
		})
	}
	pub fn get_unused_defs(&self) -> impl Iterator<Item=&AbsDef> {
		let mut used_defs = HashSet::new();
		for (_rid, r) in self.refs.iter() {
			used_defs.insert(r.ref_id);
		}
		let root = self.root.clone().unwrap_or_else(PathBuf::new);
		let mute_spans_cache = MuteSpansCache::new(root.as_path());
		let mut unused_defs = self.defs.par_iter().filter_map(|(did, d)| {
			if used_defs.contains(&did) {
				return None;
			}
			// Anything starting with _ can be unused without warning.
			if d.name.starts_with("_") {
				return None;
			}
			// Self may be unused without warning.
			if d.kind == "Local" && d.name == "self" {
				return None;
			}
			// Forbid locals for now as
			// a) the rustc lints should already catch them and
			// b) there is a false positive bug that affects them:
			// https://github.com/rust-lang/rust/issues/61385
			if d.kind == "Local" {
				return None;
			}
			// There is an id mismatch bug in rustc's save-analysis
			// output.
			// https://github.com/rust-lang/rust/issues/61302
			if d.kind == "TupleVariant" {
				return None;
			}
			// Record implementations of traits etc as used if the trait's
			// function is used
			if let Some(decl_id) = d.decl_id {
				// Whether the trait's fn is used somewhere
				let fn_in_trait_used = used_defs.contains(&decl_id);
				// Whether the trait is from another crate
				let fn_in_trait_foreign = !self.covered_crates.contains(&decl_id.krate);
				if fn_in_trait_used || fn_in_trait_foreign {
					return None;
				}
			}
			if let Some(parent) = d.parent.as_ref().and_then(|p| self.defs.get(p)) {
				// It seems that rustc doesn't emit any refs for assoc. types
				if parent.kind == "Trait" && d.kind == "Type" {
					return None;
				}
			}
			// Macros have poor save-analysis support atm:
			// https://github.com/rust-lang/rust/issues/49178#issuecomment-375454487
			// Most importantly, their spans are not emitted.
			if mute_spans_cache.is_in_macro(d.id.krate, &d.span).unwrap_or(false) {
				return None;
			}
			Some(d)
		}).collect::<Vec<_>>();
		unused_defs.sort();
		unused_defs.into_iter()
	}
}
