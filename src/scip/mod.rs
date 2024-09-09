use protobuf::Message;
use scip::{symbol::parse_symbol, types::{symbol_information, Index, Symbol, SymbolRole}};

use crate::{StrErr, Options};
use core::{cmp::Ordering, fmt::{Debug, Formatter}, write};
use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}, process::Command, sync::Arc};

fn parse_scip_index(path: &Path) -> Result<Index, StrErr> {
	println!("parsing {path:?}");
	let mut file = std::fs::File::open(path)?;
	let index = Index::parse_from_reader(&mut file)?;
	Ok(index)
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
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
	for sym in &index.external_symbols {
		let symbol = parse_symbol(&sym.symbol).unwrap();
		let symbol_short = shorten_symbol(&symbol);
		println!("  ext sym '{}' kind '{:?}' {}", sym.display_name, sym.kind, symbol_short);
		for rel in &sym.relationships {
			println!("    {:?}", rel);
		}
	}
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

#[derive(Clone, PartialEq, Eq, Copy)]
pub struct Kind(symbol_information::Kind);

impl Kind {
	pub fn kind_enum(&self) -> symbol_information::Kind {
		self.0
	}
}
impl Debug for Kind {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "{:?}", self.0)
	}
}
impl PartialOrd for Kind {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		(self.0 as i32).partial_cmp(&(other.0 as i32))
	}
}
impl Ord for Kind {
	fn cmp(&self, other: &Self) -> Ordering {
		(self.0 as i32).cmp(&(other.0 as i32))
	}
}

#[derive(Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct AbsDef {
	pub span: Span,
	pub name: Option<String>,
	pub kind: Option<Kind>,
}

pub struct AnalysisDb {
	options :Options,
	root :Option<PathBuf>,
	index: Index,
	definitions: HashMap<String, AbsDef>,
}

impl AnalysisDb {
	pub fn from_path(path :&str, options :Options) -> Result<Self, StrErr> {
		let path = Path::new(path);
		let index = parse_scip_index(path)?;
		info!("parsed scip file. found {} documents", index.documents.len());
		let root = path.parent()
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.and_then(|p| p.parent())
			.map(|p| p.to_owned());
		let mut definitions = HashMap::new();
		for doc in &index.documents {
			let sym_name_kinds = doc.symbols.iter()
				.map(|sym| {
					(sym.symbol.clone(), (sym.display_name.clone(), sym.kind.enum_value().ok()))
				})
				.collect::<HashMap<_,_>>();
			let path_arc: Arc<str> = Arc::from(doc.relative_path.clone().into_boxed_str());
			for occ in &doc.occurrences {
				if occ.symbol_roles & SymbolRole::Definition as i32 == 0 {
					continue;
				}
				let name_kind = sym_name_kinds.get(&occ.symbol);
				let abs_def = AbsDef {
					span: Span::from_scip_range(&path_arc, &occ.range)?,
					name: name_kind.map(|(name, _kind)| name.clone()),
					kind: name_kind.and_then(|(_name, kind)| kind.map(|kind| Kind(kind)).clone()),
				};
				let symbol = parse_symbol(&occ.symbol).unwrap();
				trace!("Adding def {}", shorten_symbol(&symbol));
				definitions.insert(occ.symbol.clone(), abs_def);
			}
		}
		Ok(AnalysisDb {
			options,
			root,
			index,
			definitions,
		})
	}
	pub fn dump_index(&self) -> Result<(), StrErr> {
		dump_index(&self.index)
	}
	pub fn get_unused_defs(&self) -> impl Iterator<Item=AbsDef> {
		let mut used_defs = HashSet::new();
		for doc in &self.index.documents {
			for occ in &doc.occurrences {
				if occ.symbol_roles & SymbolRole::Definition as i32 != 0 {
					// Definitions we skip, as those don't count as use
					continue;
				}
				let symbol = parse_symbol(&occ.symbol).unwrap();
				trace!("Adding used def {}", shorten_symbol(&symbol));
				used_defs.insert(occ.symbol.clone());
			}
		}
		let mut unused_defs = self.definitions.iter()
			.filter(|(sym, def)| {
				if used_defs.get(sym.as_str()).is_some() {
					return false;
				}
				// Anything starting with _ can be unused without warning.
				if def.name.as_ref().map(|name| name.starts_with("_")).unwrap_or_default() {
					return false;
				}
				true
			})
			.map(|(_sym, def)| def.clone())
			.collect::<Vec<_>>();
		unused_defs.sort();
		unused_defs.into_iter()
	}
}

pub fn run_scip(dir: &Path, output_file: &Path) -> Result<(), StrErr> {
	let mut process = Command::new("rust-analyzer")
		.arg("scip")
		.arg(dir)
		.arg("--output")
		.arg(output_file)
		.spawn()?;
	let result = process.wait()?;
	if !result.success() {
		return Err(StrErr(format!("rust-analyzer command failed")));
	}
	Ok(())
}