#![doc = include_str!("../README.md")]

use thiserror::Error;

use std::process::Command;
use std::str::from_utf8;
use tracing::{debug, warn};

/// Error type for composer_parser
#[derive(Debug, Error)]
pub enum Error {
    /// This means something went wrong when we were parsing the JSON output
    /// of the program
    #[error("Error parsing JSON: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    /// This means the output of the program contained some string that was not
    /// valid UTF-8
    #[error("Error interpreting program output as UTF-8: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    /// This is likely to be an error when executing the program using std::process
    #[error("I/O Error: {0}")]
    StdIoError(#[from] std::io::Error),
}

/// These are options to modify the behaviour of the program.
#[derive(Debug, clap::Parser)]
pub struct ComposerOutdatedOptions {
    /// Dependencies that should be ignored
    #[clap(
        short = 'i',
        long = "ignore",
        value_name = "PACKAGE_NAME",
        number_of_values = 1,
        help = "Dependencies that should be ignored"
    )]
    ignored_packages: Vec<String>,
}

/// Outer structure for parsing composer-outdated output
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ComposerOutdatedData {
    /// Since we call composer outdated with --locked it returns all package
    /// information in this field
    pub locked: Vec<PackageStatus>,
}

/// Inner, per-package structure when parsing composer-outdated output
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PackageStatus {
    /// Package name
    pub name: String,
    /// Package version in use
    pub version: String,
    /// Latest package version available
    pub latest: String,
    /// Is an update required and if so, is it semver-compatible or not
    #[serde(rename = "latest-status")]
    pub latest_status: UpdateRequirement,
    /// Description for the package
    pub description: String,
    /// Further notes, e.g. if a package has been abandoned
    pub warning: Option<String>,
}

/// What kind of update, if any, is required for a package
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateRequirement {
    /// No update is required
    UpToDate,
    /// An update is required but it is semver-compatible to the version in use
    SemverSafeUpdate,
    /// An update is required to a version that is not semver-compatible
    UpdatePossible,
}

impl std::fmt::Display for UpdateRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpToDate => {
                write!(f, "up-to-date")
            }
            Self::SemverSafeUpdate => {
                write!(f, "semver-safe-update")
            }
            Self::UpdatePossible => {
                write!(f, "update-possible")
            }
        }
    }
}

/// What the exit code indicated about required updates
#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IndicatedUpdateRequirement {
    /// No update is required
    UpToDate,
    /// An update is required
    UpdateRequired,
}

impl std::fmt::Display for IndicatedUpdateRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpToDate => {
                write!(f, "up-to-date")
            }
            Self::UpdateRequired => {
                write!(f, "update-required")
            }
        }
    }
}

/// main entry point for the composer-outdated call
///
/// # Errors
///
/// fails if the call to composer outdated fails or the output can not be parsed
pub fn outdated(
    options: &ComposerOutdatedOptions,
) -> Result<(IndicatedUpdateRequirement, ComposerOutdatedData), Error> {
    let mut cmd = Command::new("composer");

    cmd.args([
        "outdated",
        "-f",
        "json",
        "--no-plugins",
        "--strict",
        "--locked",
        "-m",
    ]);

    for package_name in &options.ignored_packages {
        cmd.args(["--ignore", package_name]);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        warn!(
            "composer outdated did not return with a successful exit code: {}",
            output.status
        );
        debug!("stdout:\n{}", from_utf8(&output.stdout)?);
        if !output.stderr.is_empty() {
            warn!("stderr:\n{}", from_utf8(&output.stderr)?);
        }
    }

    let update_requirement = if output.status.success() {
        IndicatedUpdateRequirement::UpToDate
    } else {
        IndicatedUpdateRequirement::UpdateRequired
    };

    let json_str = from_utf8(&output.stdout)?;
    let data: ComposerOutdatedData = serde_json::from_str(json_str)?;
    Ok((update_requirement, data))
}

#[cfg(test)]
mod test {
    use super::*;
    //use pretty_assertions::{assert_eq, assert_ne};

    /// this test requires a composer.json and composer.lock in the main crate
    /// directory (working dir of the tests) and a composer command in PATH
    #[test]
    fn test_run_composer_outdated() -> Result<(), Error> {
        outdated(&ComposerOutdatedOptions {
            ignored_packages: vec![],
        })?;
        Ok(())
    }
}
