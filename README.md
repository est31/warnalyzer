# Warnalyzer

Remove unused code from multi-crate Rust projects.

The `dead_code` lint family of rustc is limited to one crate only and thus can't tell whether some public API is used inside a multi-crate project or not.

This tool, warnalyzer, provides unused code detection functionality for such multi-crate projects.

## Known false-positives

It's still early on. There are a couple of false positives that the tool reports:

* Any usage by macros is not seen by the tool
* Enum variants are not recognized (worked around in the code but [it would be cool to have the rustc bug fixed](https://github.com/rust-lang/rust/issues/61302))
* Implementations of a trait from a crates.io crate and then passing it to a function that requires it (I have a possible workaround in mind)
* Proc macro functions are not recognized as such and therefore get reported
* `#[allow(dead_code)]` has no effect
