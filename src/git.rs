use std::fmt;
use std::process::Command;

use clogconfig::ClogConfig;

#[derive(Clone)]
pub struct Commit<'a> {
    pub hash: String,
    pub subject: String,
    pub component: String,
    pub closes: Vec<String>,
    pub breaks: Vec<String>,
    pub commit_type: &'a String 
}

pub type Commits = Vec<Commit>;

impl<'a> fmt::Debug for Commit<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{
            hash:{:?},
            subject: {:?},
            commit_type: {:?},
            component: {:?},
            closes: {:?},
            breaks: {:?}
        }}", self.hash, self.subject, self.commit_type, self.component, self.closes, self.breaks)
    }
}

pub fn get_latest_tag() -> String {
    let output = Command::new("git")
            .arg("rev-list")
            .arg("--tags")
            .arg("--max-count=1")
            .output().unwrap_or_else(|e| panic!("Failed to run 'git rev-list' with error: {}",e));
    let buf = String::from_utf8_lossy(&output.stdout);

    buf.trim_matches('\n').to_owned()
}

pub fn get_latest_tag_ver() -> String {
    let output = Command::new("git")
            .arg("describe")
            .arg("--tags")
            .arg("--abbrev=0")
            .output().unwrap_or_else(|e| panic!("Failed to run 'git describe' with error: {}",e));

    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn get_last_commit() -> String {
    let output = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .output().unwrap_or_else(|e| panic!("Failed to run 'git rev-parse' with error: {}", e));

    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn get_log_entries(config: &ClogConfig) -> Commits {

    let range = match &config.from[..] {
        "" => "HEAD".to_owned(),
        _  => format!("{}..{}", config.from, config.to)
    };

    let output = Command::new("git")
            .arg("log")
            .arg("-E")
            .arg(&format!("--grep={}", config.grep))
            .arg(&format!("--format={}", config.format))
            .arg(&range)
            .output().unwrap_or_else(|e| panic!("Failed to run 'git log' with error: {}", e));

    String::from_utf8_lossy(&output.stdout)
            .split("\n==END==\n")
            .map(|commit_str| { parse_raw_commit(commit_str, config) })
            .filter(| entry| entry.commit_type != "Unknown")
            .collect()
}


fn parse_raw_commit<'a>(commit_str:&str, config: &'a ClogConfig) -> Commit<'a> {
    let mut lines = commit_str.split('\n');

    let hash = lines.next().unwrap_or("").to_owned();

    let commit_pattern = regex!(r"^(.*?)(?:\((.*)?\))?:(.*)");
    let (subject, component, commit_type) =
        match lines.next().and_then(|s| commit_pattern.captures(s)) {
            Some(caps) => {
                let commit_type = config.section_for(caps.at(1).unwrap_or(""));
                let component = caps.at(2);
                let subject = caps.at(3);
                (subject, component, commit_type)
           },
           None => (Some(""), Some(""), config.section_for("unk"))
        };
    let closes_pattern = regex!(r"(?:Closes|Fixes|Resolves)\s((?:#(\d+)(?:,\s)?)+)");
    let closes = lines.filter_map(|line| closes_pattern.captures(line))
                      .map(|caps| caps.at(2).unwrap_or("").to_owned())
                      .collect();

    Commit {
        hash: hash,
        subject: subject.unwrap().to_owned(),
        component: component.unwrap_or("").to_owned(),
        closes: closes,
        breaks: vec![],
        commit_type: commit_type
    }
}
