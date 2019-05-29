use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct CrateId {
	name :String,
	disambiguator :(u64, u64),
}

#[derive(Deserialize, Debug)]
pub struct ItemId {
	krate :u32,
	index :u32,
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
	num :u32,
	id :CrateId,
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
