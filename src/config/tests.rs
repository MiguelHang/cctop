use std::path::Path;
use std::time::Duration;

use super::{Config, Sort};
use crate::test_util::TempDir;

fn parse(text: &str) -> super::Loaded {
    Config::parse(text, Path::new("/conf/config.toml"))
}

mod loading {
    use super::*;

    #[test]
    fn a_missing_file_gives_the_defaults_without_warnings() {
        let dir = TempDir::new();
        let loaded = Config::load(&dir.path().join("config.toml"));
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.warnings, Vec::<String>::new());
    }

    #[test]
    fn reads_the_file() {
        let dir = TempDir::new();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "sort = \"cpu\"\n").expect("write config");
        assert_eq!(Config::load(&path).config.sort, Sort::Cpu);
    }

    #[test]
    fn an_unreadable_file_gives_the_defaults_and_a_warning() {
        let dir = TempDir::new();
        let loaded = Config::load(dir.path());
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.warnings.len(), 1);
        assert!(loaded.warnings[0].starts_with("could not read"), "{:?}", loaded.warnings);
    }
}

mod parsing {
    use super::*;

    #[test]
    fn the_defaults_are_the_documented_ones() {
        let config = Config::default();
        assert_eq!(config.interval, Duration::from_secs(2));
        assert_eq!(config.memory_threshold, 4 << 30);
        assert!((config.cpu_threshold - 300.0).abs() < f64::EPSILON);
        assert_eq!(config.cpu_sustain, Duration::from_secs(60));
        assert_eq!(config.alert_interval, Duration::from_secs(5));
        assert_eq!(config.sort, Sort::Memory);
    }

    #[test]
    fn an_empty_file_gives_the_defaults() {
        let loaded = parse("");
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.warnings, Vec::<String>::new());
    }

    #[test]
    fn a_partial_file_changes_only_its_keys() {
        let loaded = parse("interval = \"1s\"\nmemory_threshold = \"512MB\"\n");
        let expected = Config { interval: Duration::from_secs(1), memory_threshold: 512_000_000, ..Config::default() };
        assert_eq!(loaded.config, expected);
        assert_eq!(loaded.warnings, Vec::<String>::new());
    }

    #[test]
    fn every_key_can_be_set() {
        let text = "interval = \"3s\"\nmemory_threshold = \"8GiB\"\ncpu_threshold = 150\n\
                    cpu_sustain = \"2m\"\nalert_interval = \"10s\"\nsort = \"cpu\"\n";
        let expected = Config {
            interval: Duration::from_secs(3),
            memory_threshold: 8 << 30,
            cpu_threshold: 150.0,
            cpu_sustain: Duration::from_secs(120),
            alert_interval: Duration::from_secs(10),
            sort: Sort::Cpu,
        };
        assert_eq!(parse(text).config, expected);
    }

    #[test]
    fn unknown_keys_are_ignored_silently() {
        let loaded = parse("colour = \"red\"\n[theme]\nx = 1\n");
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.warnings, Vec::<String>::new());
    }
}

mod invalid_values {
    use super::*;

    #[test]
    fn keep_the_default_of_that_key_and_warn() {
        let loaded = parse("interval = \"x\"\nsort = \"cpu\"\n");
        assert_eq!(loaded.config, Config { sort: Sort::Cpu, ..Config::default() });
        assert_eq!(
            loaded.warnings,
            [
                "interval = \"x\" is not valid, expected a duration between 100ms and 24h such as \"2s\"; using the default"
            ]
        );
    }

    #[test]
    fn warn_once_per_key() {
        let loaded = parse("memory_threshold = \"lots\"\ncpu_threshold = -1\nsort = 3\n");
        assert_eq!(loaded.config, Config::default());
        let keys: Vec<_> = loaded.warnings.iter().filter_map(|w| w.split(' ').next()).collect();
        assert_eq!(keys, ["memory_threshold", "cpu_threshold", "sort"]);
    }

    #[test]
    fn a_file_that_is_not_toml_gives_the_defaults_and_a_warning() {
        let loaded = parse("sort = \"cpu\"\ninterval = \n");
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.warnings.len(), 1);
        assert!(
            loaded.warnings[0].starts_with("/conf/config.toml is not valid TOML at line 2 ("),
            "{:?}",
            loaded.warnings
        );
    }
}
