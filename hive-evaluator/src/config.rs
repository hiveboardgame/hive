use crate::scheme::Scheme;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};

/// The engines this worker can search with, read from a TOML file; see the README, "Engines".
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    /// The engine to run when there are several.
    #[serde(rename = "use")]
    chosen: Option<String>,
    engines: BTreeMap<String, EngineConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineConfig {
    pub command: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    pub dir: Option<PathBuf>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// The analysis command, with `{budget}` where the search budget goes.
    pub analyze: String,
    /// Engines in all, or per helper when there are helpers.
    #[serde(default = "one")]
    pub count: u16,
    /// Files whose contents tell one version of the engine from another, such as its weights.
    #[serde(default)]
    pub fingerprint: Vec<PathBuf>,
    /// The budget every position is searched with first.
    pub screen: u32,
    /// The budget the moves that may have lost are searched with again.
    pub full: u32,
    #[serde(default = "default_threshold")]
    pub threshold: f32,
    pub helper: Option<HelperConfig>,
}

/// A process the engines talk to, such as a server holding a net; each gets its own port.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelperConfig {
    pub command: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    pub dir: Option<PathBuf>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Text in a line of its output that says it is ready.
    pub ready: String,
    #[serde(default = "default_ready_timeout_secs")]
    pub ready_timeout_secs: u64,
    #[serde(default = "one")]
    pub count: u16,
    pub first_port: u16,
}

fn one() -> u16 {
    1
}

fn default_threshold() -> f32 {
    2.0
}

/// Long enough for a GPU server that compiles its net first.
fn default_ready_timeout_secs() -> u64 {
    900
}

impl EngineConfig {
    /// The engine named `chosen`, else the file's `use`, else its only engine, with its name.
    pub fn load(path: &Path, chosen: Option<&str>) -> Result<(String, Self)> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading the engine config {}", path.display()))?;
        Self::pick(&text, chosen).with_context(|| format!("in {}", path.display()))
    }

    pub(crate) fn pick(text: &str, chosen: Option<&str>) -> Result<(String, Self)> {
        let file: ConfigFile = toml::from_str(text)?;
        let names = || file.engines.keys().cloned().collect::<Vec<_>>().join(", ");
        let name = match chosen.map(str::to_string).or(file.chosen.clone()) {
            Some(name) => name,
            None if file.engines.len() == 1 => names(),
            None if file.engines.is_empty() => bail!("no [engines.<name>] configured"),
            None => bail!("several engines ({}) and no `use = \"<name>\"`", names()),
        };
        let engine = file
            .engines
            .get(&name)
            .cloned()
            .ok_or_else(|| anyhow!("no engine named {name}; there is {}", names()))?;
        if !engine.analyze.contains("{budget}") {
            bail!("engines.{name}.analyze has no {{budget}}");
        }
        if engine.count == 0 || engine.helper.as_ref().is_some_and(|h| h.count == 0) {
            bail!("engines.{name}: a count of 0 leaves nothing to search with");
        }
        Ok((name, engine))
    }

    pub fn scheme(&self) -> Scheme {
        Scheme {
            screen: self.screen,
            threshold: self.threshold,
            full: self.full,
        }
    }
}

impl HelperConfig {
    pub fn ready_timeout(&self) -> Duration {
        Duration::from_secs(self.ready_timeout_secs)
    }
}

/// `args` with `{port}` filled in, when there is a port to fill in.
pub fn with_port(args: &[String], port: Option<u16>) -> Vec<String> {
    args.iter()
        .map(|arg| match port {
            Some(port) => arg.replace("{port}", &port.to_string()),
            None => arg.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO: &str = r#"
        use = "net"

        [engines.net]
        command = "/opt/net-engine/bin/engine"
        args = ["--server-port={port}", "--threads=1"]
        dir = "/opt/net-engine"
        env = { ENGINE_LOG = "quiet" }
        analyze = "analyze sims {budget}"
        fingerprint = ["/opt/net-engine/weights.bin"]
        screen = 64
        full = 800
        threshold = 3.0

        [engines.net.helper]
        command = "/opt/net-engine/bin/server"
        args = ["--port", "{port}"]
        ready = "READY"
        count = 3
        first_port = 41871

        [engines.classic]
        command = "classic-engine"
        analyze = "analyze depth {budget}"
        count = 4
        screen = 4
        full = 8
    "#;

    #[test]
    fn the_file_says_which_engine_runs_and_a_name_given_overrides_it() {
        let (name, net) = EngineConfig::pick(TWO, None).unwrap();
        assert_eq!(name, "net");
        assert_eq!(net.count, 1);
        assert_eq!(net.env["ENGINE_LOG"], "quiet");
        assert_eq!(
            net.scheme(),
            Scheme {
                screen: 64,
                threshold: 3.0,
                full: 800
            }
        );
        let helper = net.helper.unwrap();
        assert_eq!((helper.count, helper.first_port), (3, 41871));
        assert_eq!(helper.ready_timeout(), Duration::from_secs(900));
        assert_eq!(
            with_port(&net.args, Some(41872)),
            ["--server-port=41872", "--threads=1"]
        );

        let (name, classic) = EngineConfig::pick(TWO, Some("classic")).unwrap();
        assert_eq!(name, "classic");
        assert!(classic.helper.is_none());
        assert_eq!((classic.count, classic.threshold), (4, 2.0));
    }

    #[test]
    fn a_single_engine_needs_no_choosing() {
        let one = r#"
            [engines.only]
            command = "engine"
            analyze = "analyze {budget}"
            screen = 100
            full = 2000
        "#;
        assert_eq!(EngineConfig::pick(one, None).unwrap().0, "only");
    }

    #[test]
    fn mistakes_in_the_config_are_refused() {
        let unchosen = TWO.replace(r#"use = "net""#, "");
        assert!(EngineConfig::pick(&unchosen, None).is_err());
        assert!(EngineConfig::pick(TWO, Some("missing")).is_err());
        let no_budget = TWO.replace("analyze sims {budget}", "analyze sims 800");
        assert!(EngineConfig::pick(&no_budget, None).is_err());
        let typo = TWO.replace("first_port", "first_prot");
        assert!(EngineConfig::pick(&typo, None).is_err());
        let none = TWO.replace("count = 3", "count = 0");
        assert!(EngineConfig::pick(&none, None).is_err());
        assert!(EngineConfig::pick("engines = {}", None).is_err());
    }
}
