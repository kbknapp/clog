// Until regex_macros compiles with nightly, these should be commented out
//
// #![cfg_attr(feature = "unstable", feature(plugin))]
// #![cfg_attr(feature = "unstable", plugin(regex_macros))]

extern crate regex;
extern crate semver;
extern crate toml;

#[macro_use]
mod macros;
mod logentry;
mod git;
mod log_writer;
mod sectionmap;
mod clogconfig;