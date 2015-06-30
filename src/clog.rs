use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::fmt::Display;
use std::env;
use std::collections::HashMap;

use clap::ArgMatches;
use toml::{Value, Parser};
use semver;

use git;
use CLOG_CONFIG_FILE;

arg_enum!{
    pub enum LinkStyle {
        Github,
        Gitlab,
        Stash
    }
}

pub struct Clog {
    pub grep: String,
    pub format: String,
    pub repo: String,
    pub link_style: LinkStyle,
    pub version: String,
    pub patch_ver: bool,
    pub subtitle: String,
    pub from: String,
    pub to: String,
    pub changelog: String,
    pub section_map: HashMap<String, Vec<String>>
}

pub type ClogResult = Result<Clog, Box<Display>>;

impl Clog {
    pub fn new() -> Clog {
        let mut sections = HashMap::new();
        sections.insert("Features".to_owned(), vec!["ft".to_owned(), "feat".to_owned()]);
        sections.insert("Bug Fixes".to_owned(), vec!["fx".to_owned(), "fix".to_owned()]);
        sections.insert("Unknown".to_owned(), vec!["unk".to_owned()]);
        sections.insert("Breaks".to_owned(), vec![]);

        Clog {
            grep: format!("{}BREAKING'",
                sections.values()
                        .map(|v| v.iter().fold(String::new(), |acc, al| {
                            acc + &format!("^{}|", al)[..]
                        }))
                        .fold(String::new(), |acc, al| {
                            acc + &format!("^{}|", al)[..]
                        })),
            format: "%H%n%s%n%b%n==END==".to_owned(),
            repo: "".to_owned(),
            link_style: LinkStyle::Github,
            version: (&git::get_last_commit()[0..8]).to_owned(),
            patch_ver: false,
            subtitle: subtitle,
            from: from,
            to: "HEAD".to_owned(),
            changelog: "changelog.md".to_owned(),
            section_map: sections
        }
    }

    pub fn from_rel_file<P: AsRef<Path>>(cfg_file: P) -> ClogResult {
        let cwd = match env::current_dir() {
            Ok(d)  => d,
            Err(e) => return Err(Box::new(e)),
        };

        Clog::from_abs_file(Path::new(&cwd).join(cfg_file.as_ref()))
    }

    pub fn from_abs_file<P: AsRef<Path>>(cfg_file: P) -> ClogResult {
        let mut clog = Clog::new();

        let mut toml_from_latest = None;
        let mut toml_repo = None;
        let mut toml_subtitle = None;
        let mut toml_link_style = None;
        let mut toml_outfile = None;

        if let Ok(ref mut toml_f) = File::open(cfg_file.as_ref()){
            let mut toml_s = String::with_capacity(100);

            if let Err(e) = toml_f.read_to_string(&mut toml_s) {
                return Err(Box::new(e))
            }

            toml_s.shrink_to_fit();

            let mut toml = Parser::new(&toml_s[..]);

            let toml_table = match toml.parse() {
                Some(table) => table,
                None        => {
                    return Err(Box::new(format!("Error parsing file {}\n\nPlease check the format or specify the options manually", cfg_file)))
                }
            };

            let clog_table = match toml_table.get("clog") {
                Some(table) => table,
                None        => {
                    return Err(Box::new(format!("Error parsing file {}\n\nPlease check the format or specify the options manually", cfg_file)))
                }
            };

            toml_from_latest = clog_table.lookup("from-latest-tag").unwrap_or(&Value::Boolean(false)).as_bool();
            toml_repo = match clog_table.lookup("repository") {
                Some(val) => Some(val.as_str().unwrap_or("").to_owned()),
                None      => Some("".to_owned())
            };
            toml_subtitle = match clog_table.lookup("subtitle") {
                Some(val) => Some(val.as_str().unwrap_or("").to_owned()),
                None      => Some("".to_owned())
            };
            toml_link_style = match clog_table.lookup("link-style") {
                Some(val) => match val.as_str().unwrap_or("github").parse::<LinkStyle>() {
                    Ok(style) => Some(style),
                    Err(err)   => {
                        return Err(Box::new(format!("Error parsing file {}\n\n{}", cfg_file, err)))
                    }
                },
                None      => Some(LinkStyle::Github)
            };
            outfile = match clog_table.lookup("outfile") {
                Some(val) => Some(val.as_str().unwrap_or("changelog.md").to_owned()),
                None      => None
            };
            match toml_table.get("sections") {
                Some(table) => {
                    match table.as_table() {
                        Some(table) => {
                            for (sec, val) in table.iter() {
                                if let Some(vec) = val.as_slice() {
                                    let alias_vec = vec.iter().map(|v| v.as_str().unwrap_or("").to_owned()).collect::<Vec<_>>();
                                    clog.sections.insert(sec.to_owned(), alias_vec);
                                }
                            }
                        },
                        None        => ()
                    }
                },
                None        => ()
            };
        };

        if toml_from_latest.unwrap_or(false) {
            clog.from = git::get_latest_tag();
        }

        if let Some(repo) = toml_repo {
            clog.repo = repo;
        }

        if let Some(ls) = toml_link_style {
            clog.link_style = ls;
        }

        if let Some(subtitle) = toml_subtitle {
            clog.subtitle = subtitle;
        }

        if let Some(outfile) = toml_outfile {
            clog.changelog = outfile;
        }

        Ok(clog)
    }

    pub fn from_matches(matches: &ArgMatches) -> ConfigResult {
        // compute version early, so we can exit on error
        clog.version = {
            // less typing later...
            let (major, minor, patch) = (matches.is_present("major"), matches.is_present("minor"), matches.is_present("patch"));
            if matches.is_present("ver") {
                matches.value_of("ver").unwrap().to_owned()
            } else if major || minor || patch {
                let mut had_v = false;
                let v_string = git::get_latest_tag_ver();
                let first_char = v_string.chars().nth(0).unwrap_or(' ');
                let v_slice = if first_char == 'v' || first_char == 'V' {
                    had_v = true;
                    v_string.trim_left_matches(|c| c == 'v' || c == 'V')
                } else {
                    &v_string[..]
                };
                match semver::Version::parse(v_slice) {
                    Ok(ref mut v) => {
                        // if-else may be quicker, but it's longer mentally, and this isn't slow
                        match (major, minor, patch) {
                            (true,_,_) => { v.major += 1; v.minor = 0; v.patch = 0; },
                            (_,true,_) => { v.minor += 1; v.patch = 0; },
                            (_,_,true) => { v.patch += 1; patch_ver = true; },
                            _          => unreachable!()
                        }
                        format!("{}{}", if had_v{"v"}else{""}, v)
                    },
                    Err(e) => {
                        return Err(Box::new(format!("Error: {}\n\n\tEnsure the tag format follows Semantic Versioning such as N.N.N\n\tor set the version manually with --setversion <version>" , e )));
                    }
                }
            } else {
                clog.version
            }
        };


        let from = if let Some(from) = matches.value_of("from") {
            from.to_owned()
        } else if matches.is_present("from-latest-tag") || toml_from_latest.unwrap_or(false) {
            git::get_latest_tag()
        } else {
           "".to_owned()
        };

        let repo = match matches.value_of("repository") {
            Some(repo) => repo.to_owned(),
            None       => toml_repo.unwrap_or("".to_owned())
        };

        let link_style = value_t!(matches.value_of("link-style"), LinkStyle).unwrap_or(toml_link_style.unwrap_or(LinkStyle::Github));


        let subtitle = match matches.value_of("subtitle") {
            Some(title) => title.to_owned(),
            None        => toml_subtitle.unwrap_or("".to_owned())
        };

        if let Some(file) = matches.value_of("outfile") {
            outfile = Some(file.to_owned());
        }

        Ok(Clog {
            grep: format!("{}BREAKING'",
                sections.values()
                        .map(|v| v.iter().fold(String::new(), |acc, al| {
                            acc + &format!("^{}|", al)[..]
                        }))
                        .fold(String::new(), |acc, al| {
                            acc + &format!("^{}|", al)[..]
                        })),
            format: "%H%n%s%n%b%n==END==".to_owned(),
            repo: repo,
            link_style: link_style,
            version: version,
            patch_ver: patch_ver,
            subtitle: subtitle,
            from: from,
            to: matches.value_of("to").unwrap_or("HEAD").to_owned(),
            changelog: outfile.unwrap_or("changelog.md".to_owned()),
            section_map: sections
        })
    }

    pub fn section_for(&self, alias: &str) -> &String {
        self.section_map.iter().filter(|&(_, v)| v.iter().any(|s| s == alias)).map(|(k, _)| k).next().unwrap_or(self.section_map.keys().filter(|&k| *k == "Unknown".to_owned()).next().unwrap())
    }
}
