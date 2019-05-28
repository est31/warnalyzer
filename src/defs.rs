use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct CrateId {
	name :String,
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
pub struct CrateSaveAnalysis {
	pub prelude :Prelude,
}
