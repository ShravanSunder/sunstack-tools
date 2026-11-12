#![allow(clippy::unwrap_used)]

use super::GithubHeadingSlugger;
use serde::Deserialize;

#[derive(Deserialize)]
struct UpstreamFixture {
    name: String,
    input: String,
    expected: String,
}

#[test]
fn matches_github_slugger_2_0_0_fixtures_in_sequence() {
    let fixtures: Vec<UpstreamFixture> =
        serde_json::from_str(include_str!("github_heading_slug_test/fixtures.json")).unwrap();
    assert_eq!(fixtures.len(), 78);
    let mut slugger = GithubHeadingSlugger::new();

    for fixture in fixtures {
        assert_eq!(
            slugger.slug(&fixture.input),
            fixture.expected,
            "{}",
            fixture.name
        );
    }
}

#[test]
fn resolves_collisions_with_existing_numbered_slugs_and_empty_slugs() {
    let mut slugger = GithubHeadingSlugger::new();
    let inputs = ["foo", "foo-1", "foo", "foo-1", "", "😄", "🫠"];
    let actual: Vec<_> = inputs.iter().map(|input| slugger.slug(input)).collect();

    assert_eq!(actual, ["foo", "foo-1", "foo-2", "foo-1-1", "", "-1", "-2"]);
}

#[test]
fn preserves_each_space_and_uses_contextual_unicode_lowercase() {
    let mut slugger = GithubHeadingSlugger::new();

    assert_eq!(slugger.slug("  A  B  "), "--a--b--");
    assert_eq!(slugger.slug("ΟΣ"), "ος");
    assert_eq!(slugger.slug("İ"), "i\u{307}");
}

#[test]
fn characterizes_javascript_and_rust_lowercase_differences() {
    let fixtures: Vec<(String, String)> = serde_json::from_str(include_str!(
        "github_heading_slug_test/lowercase-scalars.json"
    ))
    .unwrap();
    assert_eq!(fixtures.len(), 1508);
    let mut seen = std::collections::HashSet::new();
    let rust_unicode = std::char::UNICODE_VERSION;
    let rustc_version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "rustc version unavailable".to_owned());
    eprintln!(
        "Lowercase characterization: PATH {rustc_version}, compiled Rust Unicode {}.{}.{}, Node v26.13.0 Unicode 18.0",
        rust_unicode.0, rust_unicode.1, rust_unicode.2
    );

    let mut mismatches = 0;
    for (input, expected) in fixtures {
        let mut characters = input.chars();
        let character = characters.next().unwrap();
        assert!(characters.next().is_none(), "fixture input is one scalar");
        assert!(seen.insert(character), "fixture input is unique: {input:?}");
        let mut slugger = GithubHeadingSlugger::new();
        let actual = slugger.slug(&input);
        if actual != expected {
            mismatches += 1;
            eprintln!(
                "U+{:04X} input={input:?} Rust={actual:?} Node={expected:?}",
                u32::from(character)
            );
        }
    }
    eprintln!("Lowercase characterization: {mismatches} mismatches in 1508 scalars");
}
