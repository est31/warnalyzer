## Warnalyzer

Remove unused code from multi-crate Rust projects.

The `dead_code` lint family of rustc is limited to one crate only and thus can't tell whether some public API is used inside a multi-crate project or not.

This tool, warnalyzer, provides unused code detection functionality for such multi-crate projects.

### Usage

* Navigate to the project you want to analyze and run `RUSTFLAGS="-Z save-analysis" cargo +nightly check`.
* This command puts save analysis data into a path like `target/debug/deps/save-analysis/cratename-longhash.json`.
* Then, from the warnalyzer repo, do `cargo run <path-to-json>`
* It will list any items that it thinks weren't used.
* Make sure that you chose the json of the leaf crate. Using any other json file won't give you the full list of unused code.

### Requirements

Nightly rust is required, as [save-analysis is unstable](https://github.com/rust-lang/rust/issues/43606).

### Known bugs

It's still early on. There are a couple of bugs of the tool.

#### false-positives

These are the false positives known to me:

* Any usage by macros is not seen by the tool (save-analysis [has bad macro support](https://github.com/rust-lang/rust/issues/49178#issuecomment-375454487))
* Proc macro functions are not recognized as such and therefore get reported
* `#[allow(dead_code)]` has no effect

### Other bugs

* Enum variants are not recognized (worked around in the code but [it would be cool to have the rustc bug fixed](https://github.com/rust-lang/rust/issues/61302))
* No recursion like the `dead_code` lint of rustc, so if something only gets used by unused code, it doesn't get reported while it should.

### License
[license]: #license

This crate is distributed under the terms of both the MIT license
and the Apache License (Version 2.0), at your option.

See [LICENSE](LICENSE) for details.

#### License of your contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
