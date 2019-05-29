use serde::Deserialize;

#[derive(Deserialize, Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct CrateDisambiguator(pub u64, pub u64);

#[derive(Deserialize, Debug)]
pub struct CrateId {
	pub name :String,
	pub disambiguator :CrateDisambiguator,
}

#[derive(Deserialize, Debug)]
pub struct ItemId {
	pub krate :u32,
	pub index :u32,
}

#[derive(Deserialize, Debug)]
pub struct Span {
	pub file_name :String,
	pub line_start :u32,
	pub line_end :u32,
	pub column_start :u32,
	pub column_end :u32,
}

#[derive(Deserialize, Debug)]
pub struct ExternalCrate {
	pub num :u32,
	pub id :CrateId,
}

#[derive(Deserialize, Debug)]
pub struct Prelude {
	pub crate_id :CrateId,
	pub external_crates :Vec<ExternalCrate>,
}

#[derive(Deserialize, Debug)]
pub struct Def {
	pub kind :String,
	pub name :String,
	pub id :ItemId,
	pub span :Span,
}

#[derive(Deserialize, Debug)]
pub struct Ref {
	pub kind :String,
	pub ref_id :ItemId,
	pub span :Span,
}

#[derive(Deserialize, Debug)]
pub struct CrateSaveAnalysis {
	pub prelude :Prelude,
	pub defs :Vec<Def>,
	pub refs :Vec<Ref>,
}
