#[macro_use]
extern crate clap;
extern crate time;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::collections::BTreeMap;

use clap::{App, Arg, ArgGroup};

use log_writer::LogWriter;
use clogconfig::{LinkStyle, ClogConfig};
use sectionmap::SectionMap;

// for now the clog configuration file is .clog.toml (perhaps change to user definable
// in a future version...)
const CLOG_CONFIG_FILE: &'static str = ".clog.toml";

fn main () {
    let styles = LinkStyle::variants();
    let matches = App::new("clog")
        // Pull version from Cargo.toml
        .version(&format!("v{}", crate_version!())[..])
        .about("a conventional changelog for the rest of us")
        .args_from_usage("-r, --repository=[repo]   'Repo used for link generation (without the .git, e.g. https://github.com/thoughtram/clog)'
                          -f, --from=[from]         'e.g. 12a8546'
                          -M, --major               'Increment major version by one (Sets minor and patch to 0)'
                          -m, --minor               'Increment minor version by one (Sets patch to 0)'
                          -p, --patch               'Increment patch version by one'
                          -s, --subtitle=[subtitle] 'e.g. \"Crazy Release Title\"'
                          -t, --to=[to]             'e.g. 8057684 (Defaults to HEAD when omitted)'
                          -o, --outfile=[outfile]   'Where to write the changelog (Defaults to \'changelog.md\')'
                          -c, --config              'The Clog Configuration TOML file to use (Defaults to \'.clog.toml\')'
                          --setversion=[ver]        'e.g. 1.0.1'")
        // Because --from-latest-tag can't be used with --from, we add it seperately so we can
        // specify a .mutually_excludes()
        .arg(Arg::from_usage("-F, --from-latest-tag 'use latest tag as start (instead of --from)'")
                .conflicts_with("from"))
        // Because we may want to add more "flavors" at a later date, we can automate the process
        // of enumerating all possible values with clap
        .arg(Arg::from_usage("-l, --link-style=[style]     'The style of repository link to generate (Defaults to github)'")
            .possible_values(&styles))
        // Since --setversion shouldn't be used with any of the --major, --minor, or --match, we
        // set those as exclusions
        .arg_group(ArgGroup::with_name("setver")
                .add_all(vec!["major", "minor", "patch", "ver"]))
        .get_matches();

    let start_nsec = time::get_time().nsec;

    let clog_config = ClogConfig::from_matches(&matches).unwrap_or_else(|e| { println!("{}",e); std::process::exit(1); });

    let commits = git::get_log_entries(&clog_config);

    let sm = SectionMap::from_entries(commits);

    let mut contents = String::new();

    File::open(&Path::new(&clog_config.changelog[..])).map(|mut f| f.read_to_string(&mut contents).ok()).ok();

    let mut file = File::create(&Path::new(&clog_config.changelog[..])).ok().unwrap();
    let mut writer = LogWriter::new(&mut file, &clog_config);

    writer.write_header().ok().expect("failed to write header");
    for (sec, secmap) in sm.sections {
        writer.write_section(&sec[..], &secmap.iter().collect::<BTreeMap<_,_>>()).ok().expect(&format!("failed to write {}", sec)[..]);
    }
    // writer.write_section("Bug Fixes", &sections.fixes).ok().expect("failed to write bugfixes");
    // writer.write_section("Features", &sections.features).ok().expect("failed to write features");
    writer.write(&contents[..]).ok().expect("failed to write contents");

    let end_nsec = time::get_time().nsec;
    let elapsed_mssec = (end_nsec - start_nsec) / 1000000;
    println!("changelog updated. (took {} ms)", elapsed_mssec);
}
