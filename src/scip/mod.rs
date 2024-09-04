use protobuf::Message;
use scip::{symbol::parse_symbol, types::{Index, Symbol, SymbolRole}};

use crate::{StrErr, Options};
use std::{collections::HashSet, path::{Path, PathBuf}, sync::Arc};

pub struct AnalysisDb {
	options :Options,
	root :Option<PathBuf>,
	index: Index,
}

fn parse_scip_index(path: &Path) -> Result<Index, StrErr> {
	println!("parsing {path:?}");
	let mut file = std::fs::File::open(path)?;
	let index = Index::parse_from_reader(&mut file)?;
	Ok(index)
}

#[derive(Debug)]
pub struct Span {
	pub file: Arc<str>,
	pub start_line: u32,
	pub start_col: u32,
	pub end_line: u32,
	pub end_col: u32,
}

impl Span {
	fn from_scip_range(file: &Arc<str>, range: &[i32]) -> Result<Span, StrErr> {
		// https://docs.rs/scip/latest/scip/types/struct.Occurrence.html#structfield.range
		let range_one_based = range.iter().map(|v| *v as u32 + 1).collect::<Vec<_>>();
		let span = match &range_one_based[..] {
			&[start_line, start_col, end_line, end_col] => {
				Span {
					file: file.clone(),
					start_line,
					start_col,
					end_line,
					end_col,
				}
			}
			&[line, start_col, end_col] => {
				Span {
					file: file.clone(),
					start_line: line,
					start_col,
					end_line: line,
					end_col,
				}
			}
			_ => {
				Err(format!("range has wrong number of arguments: {}", range.len()))?
			}
		};
		Ok(span)
	}
	pub fn display_str(&self) -> String {
		format!("{}:{}:{}", self.file, self.start_line, self.start_col)
	}
}

pub struct Roles(i32);

impl Roles {
	// https://docs.rs/scip/latest/scip/types/enum.SymbolRole.html
	pub fn is_definition(&self) -> bool {
		self.0 & SymbolRole::Definition as i32 > 0
	}
	pub fn is_import(&self) -> bool {
		self.0 & SymbolRole::Import as i32 > 0
	}
	pub fn is_write_access(&self) -> bool {
		self.0 & SymbolRole::WriteAccess as i32 > 0
	}
	pub fn is_read_access(&self) -> bool {
		self.0 & SymbolRole::ReadAccess as i32 > 0
	}
	pub fn is_generated(&self) -> bool {
		self.0 & SymbolRole::Generated as i32 > 0
	}
	pub fn is_test(&self) -> bool {
		self.0 & SymbolRole::Test as i32 > 0
	}
	pub fn is_forward_definition(&self) -> bool {
		self.0 & SymbolRole::ForwardDefinition as i32 > 0
	}
}

fn shorten_symbol(symbol: &Symbol) -> String {
	let package_name = &symbol.package.name;
	let descriptors = symbol.descriptors.iter()
		.map(|d| format!("{}_{:?}", d.name, d.suffix))
		.collect::<Vec<_>>();
	let descriptor_str = descriptors.join(":");
	format!("{package_name}::{descriptor_str}")
}

fn dump_index(index: &Index) -> Result<(), StrErr> {
	println!("index absolute path: {}", index.metadata.project_root);
	for doc in &index.documents {
		let path_arc: Arc<str> = Arc::from(doc.relative_path.clone().into_boxed_str());
		println!("path: {}", doc.relative_path);
		for sym in &doc.symbols {
			let symbol = parse_symbol(&sym.symbol).unwrap();
			let symbol_short = shorten_symbol(&symbol);
			println!("  sym '{}' kind '{:?}' {}", sym.display_name, sym.kind, symbol_short);
			for rel in &sym.relationships {
				println!("    {:?}", rel);
			}
		}
		for occ in &doc.occurrences {
			let sp = Span::from_scip_range(&path_arc, &occ.range)?;
			let symbol = parse_symbol(&occ.symbol).unwrap();
			let symbol_short = shorten_symbol(&symbol);
			println!("  occ '{}' span '{}' roles {}", symbol_short, sp.display_str(), occ.symbol_roles);

		}
	}
	Ok(())
}

pub struct AbsDef {
	pub span: Span,
	pub name: Option<String>,
	pub kind: Option<String>,
}

impl AnalysisDb {
	pub fn from_path(path :&str, options :Options) -> Result<Self, StrErr> {
		let path = Path::new(path);
		let index = parse_scip_index(path)?;
		println!("parsed scip file. {} many documents", index.documents.len());
		let root = path.parent()
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.and_then(|p| p.parent())
		.map(|p| p.to_owned());
		Ok(AnalysisDb {
			options,
			root,
			index,
		})
	}
	pub fn dump_index(&self) -> Result<(), StrErr> {
		dump_index(&self.index)
	}
	pub fn get_unused_defs(&self) -> impl Iterator<Item=AbsDef> {
		let unused_defs = HashSet::new();
		unused_defs.into_iter()
		/*
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
			if let Some(decl_id) = d.decl_id {
				if self.options.recurse {
					// Record implementations of traits etc as used if the trait's
					// function is used

					// Whether the trait's fn is used somewhere
					let fn_in_trait_used = used_defs.contains(&decl_id);
					// Whether the trait is from another crate
					let fn_in_trait_foreign = !self.covered_crates.contains(&decl_id.krate);
					if fn_in_trait_used || fn_in_trait_foreign {
						return None;
					}
				} else {
					// Don't do any recursion
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
		*/
	}
}
