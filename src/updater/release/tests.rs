use super::*;
use serde_json::{json, Value};

fn asset_url(tag: &str) -> String {
    let (owner, repo) = github_repo().unwrap();
    format!("https://github.com/{owner}/{repo}/releases/download/{tag}/{RELEASE_ASSET_NAME}")
}

fn fixture(tag: &str) -> Value {
    json!({
        "tag_name": tag,
        "draft": false,
        "prerelease": false,
        "assets": [{
            "name": RELEASE_ASSET_NAME,
            "browser_download_url": asset_url(tag),
            "size": 3,
            "digest": "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        }]
    })
}

fn descriptor(value: Value, current: &str) -> Result<Option<ReleaseDescriptor>, String> {
    release_descriptor(serde_json::from_value(value).unwrap(), current)
}

#[test]
fn this_build_has_a_fork_version_the_updater_can_order() {
    // A version bump that breaks the `X.Y.Z-trelowney.N` scheme would silently
    // stop every future update from being offered, so guard it here.
    let current = parse_version(env!("CARGO_PKG_VERSION")).unwrap();
    assert!(fork_version(&current).is_some());
    assert!(descriptor(fixture(&format!("v{current}")), env!("CARGO_PKG_VERSION"))
        .unwrap()
        .is_none());
}

#[test]
fn selects_exact_asset_even_after_an_unrelated_executable() {
    let mut value = fixture("v2.16.0-trelowney.1");
    let mut unrelated = value["assets"][0].clone();
    unrelated["name"] = json!("unrelated.exe");
    value["assets"].as_array_mut().unwrap().insert(0, unrelated);
    let release = descriptor(value, "2.15.14-trelowney.1").unwrap().unwrap();
    assert_eq!(release.latest_version, "2.16.0-trelowney.1");
    assert!(release.asset_url.ends_with(RELEASE_ASSET_NAME));
}

#[test]
fn rejects_missing_wrong_case_and_upstream_assets() {
    for name in [
        "other.exe",
        "CLAUDE-USAGE-MONITOR-TRELOWNEY.EXE",
        "claude-code-usage-monitor.exe",
        "app.zip",
    ] {
        let mut value = fixture("v2.16.0-trelowney.1");
        value["assets"][0]["name"] = json!(name);
        assert!(descriptor(value, "2.15.14-trelowney.1")
            .unwrap_err()
            .contains("missing"));
    }
    let mut value = fixture("v2.16.0-trelowney.1");
    value["assets"] = json!([]);
    assert!(descriptor(value, "2.15.14-trelowney.1").is_err());
}

#[test]
fn rejects_duplicate_exact_assets() {
    let mut value = fixture("v2.16.0-trelowney.1");
    let asset = value["assets"][0].clone();
    value["assets"].as_array_mut().unwrap().push(asset);
    assert!(descriptor(value, "2.15.14-trelowney.1")
        .unwrap_err()
        .contains("duplicate"));
}

#[test]
fn rejects_missing_null_and_invalid_digests() {
    for digest in [
        Value::Null,
        json!(""),
        json!("sha256:abcd"),
        json!("sha512:abcd"),
    ] {
        let mut value = fixture("v2.16.0-trelowney.1");
        value["assets"][0]["digest"] = digest;
        assert!(descriptor(value, "2.15.14-trelowney.1")
            .unwrap_err()
            .contains("SHA-256"));
    }
    let mut value = fixture("v2.16.0-trelowney.1");
    value["assets"][0].as_object_mut().unwrap().remove("digest");
    assert!(descriptor(value, "2.15.14-trelowney.1").is_err());
}

#[test]
fn rejects_oversized_and_empty_assets() {
    for size in [0, super::super::download::MAX_DOWNLOAD_BYTES + 1] {
        let mut value = fixture("v2.16.0-trelowney.1");
        value["assets"][0]["size"] = json!(size);
        assert!(descriptor(value, "2.15.14-trelowney.1")
            .unwrap_err()
            .contains("size"));
    }
}

#[test]
fn rejects_unexpected_download_locations() {
    let (owner, repo) = github_repo().unwrap();
    for url in [
        asset_url("v2.16.0-trelowney.1").replacen("https://", "http://", 1),
        format!("https://example.com/{RELEASE_ASSET_NAME}"),
        format!(
            "https://github.com/CodeZeno/Claude-Code-Usage-Monitor/releases/download/v2.16.0-trelowney.1/{RELEASE_ASSET_NAME}"
        ),
        format!(
            "https://github.com/{owner}/{repo}/releases/download/v2.15.14-trelowney.1/{RELEASE_ASSET_NAME}"
        ),
    ] {
        let mut value = fixture("v2.16.0-trelowney.1");
        value["assets"][0]["browser_download_url"] = json!(url);
        assert!(descriptor(value, "2.15.14-trelowney.1")
            .unwrap_err()
            .contains("URL"));
    }
}

#[test]
fn the_updater_follows_this_repository() {
    assert_eq!(
        github_repo().unwrap(),
        ("trelowney", "claude-code-usage-monitor")
    );
}

#[test]
fn fork_builds_are_ordered_by_base_version_then_build_number() {
    for (newer, older) in [
        ("2.15.14-trelowney.1", "2.13.43-trelowney.1"),
        ("2.15.14-trelowney.2", "2.15.14-trelowney.1"),
        ("2.15.14-trelowney.10", "2.15.14-trelowney.9"),
        ("2.16.0-trelowney.1", "2.15.14-trelowney.9"),
        ("3.0.0-trelowney.1", "2.99.99-trelowney.99"),
        ("2.15.14-trelowney.1", "2.15.14"),
        ("2.15.14-trelowney.1", "1.4.9-trelowney.9"),
    ] {
        assert!(
            descriptor(fixture(&format!("v{newer}")), older)
                .unwrap()
                .is_some(),
            "{newer} should update {older}"
        );
        assert!(
            descriptor(fixture(&format!("v{older}")), newer)
                .unwrap()
                .is_none(),
            "{older} must not replace {newer}"
        );
    }
}

#[test]
fn a_local_prerelease_build_ranks_as_its_base_version() {
    assert!(descriptor(fixture("v2.15.14-trelowney.1"), "2.15.14-dev")
        .unwrap()
        .is_some());
    assert!(descriptor(fixture("v2.15.13-trelowney.9"), "2.15.14-dev")
        .unwrap()
        .is_none());
}

#[test]
fn skips_prereleases_and_drafts_even_with_mislabelled_tags() {
    for tag in [
        "v2.16.0-beta1",
        "v2.16.0-rc.1",
        "v2.16.0-trelowney",
        "v2.16.0-trelowney.1.2",
        "v2.16.0-trelowney.x",
        "v2.16.0-other.1",
    ] {
        assert!(
            descriptor(fixture(tag), "2.15.14-trelowney.1")
                .unwrap()
                .is_none(),
            "{tag}"
        );
    }
    for flag in ["draft", "prerelease"] {
        let mut value = fixture("v2.16.0-trelowney.1");
        value[flag] = json!(true);
        assert!(descriptor(value, "2.15.14-trelowney.1").unwrap().is_none());
    }
}

#[test]
fn ignores_build_metadata_and_older_or_equal_versions() {
    for (tag, current) in [
        ("v2.15.14-trelowney.1+build.2", "2.15.14-trelowney.1+build.1"),
        ("v2.15.14-trelowney.1", "2.15.14-trelowney.1"),
        ("v2.13.43-trelowney.1", "2.15.14-trelowney.1"),
    ] {
        assert!(descriptor(fixture(tag), current).unwrap().is_none());
    }
}

#[test]
fn parses_numeric_components_without_truncation() {
    for (input, expected) in [
        ("2.15.14-trelowney.1", (2, 15, 14, 1)),
        ("v2.15.14-trelowney.1", (2, 15, 14, 1)),
        ("2.15.14", (2, 15, 14, 0)),
        ("4294967296.13.44-trelowney.4294967296", (4294967296, 13, 44, 4294967296)),
        (
            "18446744073709551615.18446744073709551615.18446744073709551615-trelowney.18446744073709551615",
            (u64::MAX, u64::MAX, u64::MAX, u64::MAX),
        ),
    ] {
        let version = fork_version(&parse_version(input).unwrap()).unwrap();
        assert_eq!(
            (version.major, version.minor, version.patch, version.build),
            expected,
            "version: {input:?}"
        );
    }
}

#[test]
fn malformed_versions_are_errors() {
    for version in [
        "",
        "invalid",
        "2",
        "2..44",
        ".13.",
        "002.013.044",
        "bad.13.44",
        "2.13.bad",
        "18446744073709551616.13.44",
        "2.18446744073709551616.44",
        "2.13.18446744073709551616",
        "vv2.14.0",
        "2.14",
        "2.14.0.1",
        "2.014.0",
        "2.14.0-",
        "2.x.0",
        "2.14.0-trelowney.01",
    ] {
        assert!(parse_version(version).is_err(), "{version}");
        assert!(descriptor(fixture(version), "2.15.14-trelowney.1").is_err());
        assert!(descriptor(fixture("v2.16.0-trelowney.1"), version).is_err());
    }
}
