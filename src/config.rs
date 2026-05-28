use fqdn::FQDN;
use fqdn_trie::FqdnTrieSet;
use log::warn;
use std::fs;
use std::path::Path;

/// A compiled set of glob patterns for matching hostnames.
pub struct RuleSet {
    pub table_name: String,
    pub set_name_v4: String,
    pub set_name_v6: String,
    patterns: FqdnTrieSet<FQDN>,
}

impl RuleSet {
    /// Read patterns from a file; blank lines and `#`-prefixed comments are ignored.
    fn from_file(table_name: &str, set_name: &str, path: &Path) -> anyhow::Result<Self> {
        let mut patterns = FqdnTrieSet::default();
        let text = fs::read_to_string(path)?;
        for line in text.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                match FQDN::from_ascii_str(line) {
                    Ok(p) => {
                        patterns.insert(p);
                    }
                    Err(e) => {
                        warn!("Skipping invalid pattern '{}': {}", line, e);
                    }
                }
            }
        }
        Ok(Self {
            table_name: table_name.to_string(),
            set_name_v4: format!("{}_v4", set_name),
            set_name_v6: format!("{}_v6", set_name),
            patterns,
        })
    }

    /// Returns `true` if `hostname` matches any loaded pattern.
    ///
    /// The match is case-insensitive; the hostname is lowercased before
    /// comparison, which is the conventional DNS representation.
    pub fn matches(&self, hostname: &str) -> bool {
        match FQDN::from_ascii_str(hostname) {
            Ok(p) => {
                let x = self.patterns.lookup(&p);
                !x.is_root()
            }
            Err(e) => {
                warn!("invalid domain name'{}': {}", hostname, e);
                false
            }
        }
    }
}

pub fn load_rules(path: &Path) -> anyhow::Result<Vec<RuleSet>> {
    let mut result = Vec::new();
    if fs::exists(path)? {
        for file_table in fs::read_dir(path)?.flatten() {
            for file_set in fs::read_dir(file_table.path())?.flatten() {
                for file_rules in fs::read_dir(file_set.path())?.flatten() {
                    result.push(RuleSet::from_file(
                        &file_table.file_name().to_string_lossy(),
                        &file_set.file_name().to_string_lossy(),
                        &file_rules.path(),
                    )?);
                }
            }
        }
    }
    Ok(result)
}
