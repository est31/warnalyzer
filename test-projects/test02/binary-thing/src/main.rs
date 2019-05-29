use library_thing::{UsedStruct, used_fn};

fn main() {
	let used = UsedStruct;
	println!("{:?}{}", used, used_fn());
}
