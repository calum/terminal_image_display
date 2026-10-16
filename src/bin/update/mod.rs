//! Keeping the command line tool up to date: `termage --update` replaces the
//! running program with the newest GitHub release, and every run tells you
//! (at most once a day, and only on a terminal) when a newer one exists.
//!
//! The decisions (which version is newer, which file to download, when to
//! check again) are plain functions; only `get`, `replace`, `run` and `check`
//! touch the outside world.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

const REPO: &str = "calum/terminal_image_display";

/// Set this to anything to switch off the once a day check.
pub const NO_CHECK_VARIABLE: &str = "TERMAGE_NO_UPDATE_CHECK";

/// Set this to a mirror (the tests do) instead of GitHub. It must serve
/// `<base>/latest` (the release JSON) and `<base>/download/<tag>/<file>`.
const BASE_VARIABLE: &str = "TERMAGE_UPDATE_BASE";

const CHECK_EVERY_SECONDS: u64 = 24 * 60 * 60;
const CHECK_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_BINARY_BYTES: u64 = 200 * 1024 * 1024;

/// A version as `major.minor.patch`; ordered the way versions are.
pub type Version = (u64, u64, u64);

/// `1.2.3` or `v1.2.3` as numbers. Anything else (including pre-releases) is `None`.
pub fn parse_version(text: &str) -> Option<Version> {
    let mut parts = text.trim().trim_start_matches('v').split('.');
    let version = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(version)
}

/// The release a GitHub `releases/latest` answer describes, from its `tag_name`.
pub fn parse_latest(json: &str) -> Option<Version> {
    let after_key = json.split_once("\"tag_name\"")?.1;
    let after_colon = after_key.split_once(':')?.1.trim_start();
    let tag = after_colon.strip_prefix('"')?.split_once('"')?.0;
    parse_version(tag)
}

/// The name of the release file for this machine, like `termage-x86_64-unknown-linux-gnu`.
pub fn asset_name(os: &str, arch: &str) -> Option<String> {
    if !matches!(arch, "x86_64" | "aarch64") {
        return None;
    }
    let target = match os {
        "linux" => format!("{arch}-unknown-linux-gnu"),
        "macos" => format!("{arch}-apple-darwin"),
        "windows" if arch == "x86_64" => format!("{arch}-pc-windows-msvc.exe"),
        _ => return None,
    };
    Some(format!("termage-{target}"))
}

fn show((major, minor, patch): Version) -> String {
    format!("{major}.{minor}.{patch}")
}

/// The one line shown when `latest` is newer than `current`.
pub fn notice(current: Version, latest: Version) -> Option<String> {
    (latest > current).then(|| {
        format!(
            "termage {} is available (you have {}). Run `termage --update` to upgrade.",
            show(latest),
            show(current)
        )
    })
}

/// When the last check was, and what it found: remembered between runs.
#[derive(Debug, PartialEq)]
pub struct Cache {
    /// Seconds since 1970 when GitHub was last asked.
    pub checked_at: u64,
    /// The newest version it reported.
    pub latest: Version,
}

impl Cache {
    /// The text kept on disk.
    pub fn render(&self) -> String {
        format!("{} {}\n", self.checked_at, show(self.latest))
    }

    /// Reads [`Cache::render`] back. A damaged file is just "never checked".
    pub fn parse(text: &str) -> Option<Cache> {
        let (when, version) = text.trim().split_once(' ')?;
        Some(Cache {
            checked_at: when.parse().ok()?,
            latest: parse_version(version)?,
        })
    }

    /// Whether it is time to ask GitHub again.
    pub fn is_stale(&self, now: u64) -> bool {
        now.saturating_sub(self.checked_at) >= CHECK_EVERY_SECONDS
    }
}

/// Where the cache lives, given a way to look up environment variables.
pub fn cache_path(var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let dir = var("XDG_CACHE_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| var("LOCALAPPDATA").map(PathBuf::from))
        .or_else(|| var("HOME").map(|home| Path::new(&home).join(".cache")))?;
    Some(dir.join("termage").join("update-check"))
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn latest_url(base: Option<&str>) -> String {
    match base {
        Some(base) => format!("{base}/latest"),
        None => format!("https://api.github.com/repos/{REPO}/releases/latest"),
    }
}

fn download_url(base: Option<&str>, latest: Version, asset: &str) -> String {
    let tag = format!("v{}", show(latest));
    match base {
        Some(base) => format!("{base}/download/{tag}/{asset}"),
        None => format!("https://github.com/{REPO}/releases/download/{tag}/{asset}"),
    }
}

#[cfg(feature = "net")]
fn get(url: &str, timeout: Option<Duration>, limit: u64) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(timeout)
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("User-Agent", concat!("termage/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|error| error.to_string())?;
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|error| error.to_string())
}

#[cfg(not(feature = "net"))]
fn get(_url: &str, _timeout: Option<Duration>, _limit: u64) -> Result<Vec<u8>, String> {
    Err("this build of termage has no network support (the `net` feature)".to_string())
}

fn fetch_latest(base: Option<&str>, timeout: Option<Duration>) -> Result<Version, String> {
    let body = get(&latest_url(base), timeout, 1024 * 1024)?;
    parse_latest(&String::from_utf8_lossy(&body))
        .ok_or_else(|| "could not read the latest version".to_string())
}

/// Puts `new` where the running program `exe` is. The running file cannot be
/// overwritten on Windows, but it can be renamed out of the way.
pub fn replace(exe: &Path, new: &[u8]) -> io::Result<()> {
    let staged = exe.with_extension("new");
    let old = exe.with_extension("old");
    fs::write(&staged, new)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))?;
    }
    let _ = fs::remove_file(&old);
    let result = fs::rename(exe, &old).and_then(|()| fs::rename(&staged, exe));
    if result.is_err() {
        // put the original back if it was moved, and leave nothing behind
        if !exe.exists() {
            let _ = fs::rename(&old, exe);
        }
        let _ = fs::remove_file(&staged);
        return result;
    }
    let _ = fs::remove_file(&old);
    Ok(())
}

/// `termage --update`: download the newest release over this program.
pub fn run(current: Version) -> Result<String, String> {
    let base = std::env::var(BASE_VARIABLE).ok();
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    let asset = asset_name(os, arch).ok_or_else(|| {
        format!("there is no release for {os} {arch}; use `cargo install termage` instead")
    })?;
    let latest = fetch_latest(base.as_deref(), None)?;
    if latest <= current {
        return Ok(format!("termage {} is already up to date", show(current)));
    }
    let url = download_url(base.as_deref(), latest, &asset);
    let binary = get(&url, None, MAX_BINARY_BYTES)?;
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    replace(&exe, &binary)
        .map_err(|error| format!("could not replace {}: {error}", exe.display()))?;
    Ok(format!(
        "updated termage {} to {}",
        show(current),
        show(latest)
    ))
}

/// The once a day reminder. Never fails and never waits long: the opt-out
/// variable or no cache directory mean "say nothing", and a failed lookup is
/// remembered like a successful one so an offline machine is not slowed down.
pub fn check(current: Version) -> Option<String> {
    if std::env::var_os(NO_CHECK_VARIABLE).is_some() {
        return None;
    }
    let path = cache_path(|name| std::env::var(name).ok())?;
    let cached = fs::read_to_string(&path)
        .ok()
        .and_then(|text| Cache::parse(&text));
    let latest = match cached {
        Some(cache) if !cache.is_stale(now()) => cache.latest,
        _ => {
            let base = std::env::var(BASE_VARIABLE).ok();
            let checked_at = now();
            let latest = fetch_latest(base.as_deref(), Some(CHECK_TIMEOUT)).unwrap_or(current);
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(&path, Cache { checked_at, latest }.render());
            latest
        }
    };
    notice(current, latest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_parse_with_or_without_a_v() {
        assert_eq!(parse_version("4.0.0"), Some((4, 0, 0)));
        assert_eq!(parse_version("v10.2.33\n"), Some((10, 2, 33)));
        assert_eq!(parse_version("4.0"), None);
        assert_eq!(parse_version("4.0.0.1"), None);
        assert_eq!(parse_version("4.0.0-beta"), None);
        assert_eq!(parse_version("latest"), None);
    }

    #[test]
    fn versions_compare_by_number_not_by_text() {
        assert!(parse_version("4.10.0") > parse_version("4.9.0"));
    }

    #[test]
    fn the_latest_version_is_read_from_the_release_json() {
        let json = r#"{"url":"x","tag_name": "v4.1.0","name":"4.1.0"}"#;
        assert_eq!(parse_latest(json), Some((4, 1, 0)));
        assert_eq!(parse_latest("{}"), None);
        assert_eq!(parse_latest(r#"{"tag_name": 4}"#), None);
        assert_eq!(parse_latest(r#"{"tag_name": "nightly"}"#), None);
        assert_eq!(parse_latest(r#"{"tag_name""#), None);
    }

    #[test]
    fn each_supported_machine_has_a_release_file() {
        let name = asset_name;
        assert_eq!(
            name("linux", "x86_64").as_deref(),
            Some("termage-x86_64-unknown-linux-gnu")
        );
        assert_eq!(
            name("linux", "aarch64").as_deref(),
            Some("termage-aarch64-unknown-linux-gnu")
        );
        assert_eq!(
            name("macos", "aarch64").as_deref(),
            Some("termage-aarch64-apple-darwin")
        );
        assert_eq!(
            name("macos", "x86_64").as_deref(),
            Some("termage-x86_64-apple-darwin")
        );
        assert_eq!(
            name("windows", "x86_64").as_deref(),
            Some("termage-x86_64-pc-windows-msvc.exe")
        );
        assert_eq!(name("windows", "aarch64"), None);
        assert_eq!(name("linux", "riscv64"), None);
        assert_eq!(name("freebsd", "x86_64"), None);
    }

    #[test]
    fn urls_point_at_github_or_at_a_mirror() {
        assert_eq!(
            latest_url(None),
            "https://api.github.com/repos/calum/terminal_image_display/releases/latest"
        );
        assert_eq!(latest_url(Some("http://h:1")), "http://h:1/latest");
        assert_eq!(
            download_url(None, (4, 1, 0), "termage-x"),
            "https://github.com/calum/terminal_image_display/releases/download/v4.1.0/termage-x"
        );
        assert_eq!(
            download_url(Some("http://h:1"), (4, 1, 0), "termage-x"),
            "http://h:1/download/v4.1.0/termage-x"
        );
    }

    #[test]
    fn only_a_newer_version_is_announced() {
        let message = notice((4, 0, 0), (4, 1, 0)).unwrap();
        assert!(message.contains("4.1.0") && message.contains("4.0.0"));
        assert!(message.contains("termage --update"));
        assert_eq!(notice((4, 1, 0), (4, 1, 0)), None);
        assert_eq!(notice((5, 0, 0), (4, 9, 9)), None);
    }

    #[test]
    fn the_cache_round_trips_and_ignores_damage() {
        let cache = Cache {
            checked_at: 1_700_000_000,
            latest: (4, 2, 0),
        };
        assert_eq!(Cache::parse(&cache.render()), Some(cache));
        assert_eq!(Cache::parse(""), None);
        assert_eq!(Cache::parse("soon 4.0.0"), None);
        assert_eq!(Cache::parse("100 four"), None);
    }

    #[test]
    fn the_cache_goes_stale_after_a_day() {
        let cache = Cache {
            checked_at: 1000,
            latest: (4, 0, 0),
        };
        assert!(!cache.is_stale(1000 + CHECK_EVERY_SECONDS - 1));
        assert!(cache.is_stale(1000 + CHECK_EVERY_SECONDS));
        // a clock set back is not "stale"
        assert!(!cache.is_stale(5));
    }

    #[test]
    fn the_cache_directory_follows_the_platform_variables() {
        fn with(pairs: &'static [(&str, &str)]) -> Option<PathBuf> {
            cache_path(move |name| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| value.to_string())
            })
        }
        let tail = Path::new("termage").join("update-check");
        assert_eq!(
            with(&[("XDG_CACHE_HOME", "/x"), ("HOME", "/h")]),
            Some(Path::new("/x").join(&tail))
        );
        assert_eq!(
            with(&[("LOCALAPPDATA", "C:/L"), ("HOME", "/h")]),
            Some(Path::new("C:/L").join(&tail))
        );
        assert_eq!(
            with(&[("XDG_CACHE_HOME", ""), ("HOME", "/h")]),
            Some(Path::new("/h").join(".cache").join(&tail))
        );
        assert_eq!(with(&[]), None);
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("termage_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn replacing_the_program_swaps_the_file_and_cleans_up() {
        let dir = temp("replace");
        let exe = dir.join("termage");
        fs::write(&exe, b"old").unwrap();

        replace(&exe, b"new").unwrap();

        assert_eq!(fs::read(&exe).unwrap(), b"new");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "no leftovers");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn replacing_a_missing_program_fails_and_leaves_nothing() {
        let dir = temp("missing");

        assert!(replace(&dir.join("termage"), b"new").is_err());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(&dir).unwrap();
    }
}
