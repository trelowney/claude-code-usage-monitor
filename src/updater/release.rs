use semver::Version;
use serde::Deserialize;

use super::download::AssetIntegrity;
use super::github_repo;

pub(super) const RELEASE_ASSET_NAME: &str = "claude-usage-monitor-trelowney.exe";

/// This fork tags releases as `vX.Y.Z-trelowney.N`: the upstream base version
/// plus a fork build number. Plain SemVer would read that suffix as a
/// prerelease (ranked below `X.Y.Z`, and skipped by a stable-only updater), so
/// fork builds are ordered by (major, minor, patch, build) instead.
const FORK_PRERELEASE_PREFIX: &str = "trelowney.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ForkVersion {
    major: u64,
    minor: u64,
    patch: u64,
    build: u64,
}

#[derive(Clone, Debug)]
pub struct ReleaseDescriptor {
    pub latest_version: String,
    pub(super) asset_url: String,
    pub(super) integrity: AssetIntegrity,
}

#[derive(Deserialize)]
pub(super) struct GitHubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

pub(super) fn release_descriptor(
    release: GitHubRelease,
    current: &str,
) -> Result<Option<ReleaseDescriptor>, String> {
    let latest = parse_version(&release.tag_name)?;
    let current = parse_version(current)?;
    if release.draft || release.prerelease {
        return Ok(None);
    }
    // Anything other than a plain or `-trelowney.N` version is a real
    // prerelease (even if mislabelled as stable in GitHub): never offer it.
    let Some(latest_order) = fork_version(&latest) else {
        return Ok(None);
    };
    if latest_order <= current_order(&current) {
        return Ok(None);
    }

    let mut matches = release
        .assets
        .iter()
        .filter(|asset| asset.name == RELEASE_ASSET_NAME);
    let asset = matches
        .next()
        .ok_or_else(|| format!("The latest release is missing {RELEASE_ASSET_NAME}."))?;
    if matches.next().is_some() {
        return Err(format!(
            "The latest release has duplicate {RELEASE_ASSET_NAME} assets."
        ));
    }

    let (owner, repo) = github_repo()?;
    let expected_url = format!(
        "https://github.com/{owner}/{repo}/releases/download/{}/{RELEASE_ASSET_NAME}",
        release.tag_name
    );
    if asset.browser_download_url != expected_url {
        return Err("The update asset URL does not match the expected GitHub release.".into());
    }
    let integrity = AssetIntegrity::new(asset.size, asset.digest.as_deref())?;

    Ok(Some(ReleaseDescriptor {
        latest_version: latest.to_string(),
        asset_url: expected_url,
        integrity,
    }))
}

fn parse_version(version: &str) -> Result<Version, String> {
    Version::parse(version.strip_prefix('v').unwrap_or(version))
        .map_err(|e| format!("Invalid release version {version:?}: {e}"))
}

/// Returns the fork ordering for a plain `X.Y.Z` (build 0) or `X.Y.Z-trelowney.N`
/// version, or `None` for any other prerelease. Build metadata is ignored.
fn fork_version(version: &Version) -> Option<ForkVersion> {
    let build = if version.pre.is_empty() {
        0
    } else {
        version
            .pre
            .as_str()
            .strip_prefix(FORK_PRERELEASE_PREFIX)?
            .parse::<u64>()
            .ok()?
    };
    Some(ForkVersion {
        major: version.major,
        minor: version.minor,
        patch: version.patch,
        build,
    })
}

/// A running build that is some other prerelease (e.g. a local `-dev` build)
/// ranks as its base version, so any fork build of that base or later is newer.
fn current_order(version: &Version) -> ForkVersion {
    fork_version(version).unwrap_or(ForkVersion {
        major: version.major,
        minor: version.minor,
        patch: version.patch,
        build: 0,
    })
}

#[cfg(test)]
mod tests;
