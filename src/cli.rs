//! Argument parsing and the normative usage errors.

use clap::error::ErrorKind;
use clap::{ArgAction, Parser};

use crate::summary::MetadataMode;

const FACTOR_RULE: &str = "--factor must be a whole number from 1 to 99 \
(the percentage of the original pixel area to keep)";

const NO_PATH_NOTE: &str =
    "note: srnk always operates on the current directory and takes no path argument";

/// The validated invocation. Immutable for the whole run.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "srnk",
    version,
    about = "Shrink the JPEGs in the current folder into a srnk/ copy folder",
    after_help = "srnk always operates on the current directory and takes no path argument."
)]
pub struct RunConfig {
    /// Percentage of the original pixel area to keep (1-99).
    #[arg(long, value_name = "N", value_parser = parse_factor)]
    pub factor: u8,

    /// Also process every subfolder beneath the current directory.
    #[arg(long, action = ArgAction::SetTrue)]
    pub recursive: bool,

    /// Remove all metadata from the resized copies.
    #[arg(long, action = ArgAction::SetTrue)]
    pub strip: bool,
}

fn parse_factor(raw: &str) -> Result<u8, String> {
    match raw.parse::<u8>() {
        Ok(value) if (1..=99).contains(&value) => Ok(value),
        _ => Err(FACTOR_RULE.to_string()),
    }
}

impl RunConfig {
    /// Per-dimension scale: `sqrt(factor / 100)`. Part of the `RunConfig` contract in
    /// data-model.md; the summary renders the same value from `geometry::scale_for`.
    #[allow(dead_code)]
    pub fn scale(&self) -> f64 {
        crate::geometry::scale_for(self.factor)
    }

    /// `Strip` iff `--strip` was passed.
    pub fn metadata_mode(&self) -> MetadataMode {
        if self.strip {
            MetadataMode::Strip
        } else {
            MetadataMode::Preserve
        }
    }
}

/// Parse the command line, exiting 2 on any usage error (nothing is written to disk).
pub fn parse() -> RunConfig {
    match RunConfig::try_parse() {
        Ok(config) => config,
        Err(err) => {
            let kind = err.kind();
            let rendered = err.render().to_string();
            let code = err.exit_code();
            let _ = err.print();
            if kind == ErrorKind::UnknownArgument || rendered.contains("unexpected argument") {
                eprintln!("{NO_PATH_NOTE}");
            }
            std::process::exit(code);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_valid_range_only() {
        assert_eq!(parse_factor("1"), Ok(1));
        assert_eq!(parse_factor("99"), Ok(99));
        assert_eq!(parse_factor("0"), Err(FACTOR_RULE.to_string()));
        assert_eq!(parse_factor("100"), Err(FACTOR_RULE.to_string()));
        assert_eq!(parse_factor("abc"), Err(FACTOR_RULE.to_string()));
        assert_eq!(parse_factor("50.5"), Err(FACTOR_RULE.to_string()));
    }

    #[test]
    fn factor_is_required_and_flags_default_to_false() {
        assert!(RunConfig::try_parse_from(["srnk"]).is_err());
        let config = RunConfig::try_parse_from(["srnk", "--factor", "25"]).unwrap();
        assert_eq!(config.factor, 25);
        assert!(!config.recursive);
        assert!(!config.strip);
        assert_eq!(config.metadata_mode(), MetadataMode::Preserve);
        assert!((config.scale() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn flags_and_positionals() {
        let config =
            RunConfig::try_parse_from(["srnk", "--factor", "50", "--recursive", "--strip"])
                .unwrap();
        assert!(config.recursive);
        assert!(config.strip);
        assert_eq!(config.metadata_mode(), MetadataMode::Strip);
        assert!(RunConfig::try_parse_from(["srnk", "--factor", "50", "./photos"]).is_err());
    }
}
