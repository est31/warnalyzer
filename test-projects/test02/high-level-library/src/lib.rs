#[derive(Debug)]
pub struct UsedStruct;

pub struct UnusedStruct;

pub fn used_fn() -> u32 {
	42
}

pub fn unused_fn() -> u32 {
	42
}
