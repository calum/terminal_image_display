//! Where an image comes from: a file on disk, or a web address.

use std::fs;
use std::path::Path;

use crate::Error;

/// Downloads bigger than this are refused, so a wrong link cannot fill memory.
#[cfg(feature = "net")]
const MAX_DOWNLOAD_BYTES: u64 = 100 * 1024 * 1024;

/// Whether `input` is a web address (`http://` or `https://`) rather than a
/// file path.
///
/// ```
/// assert!(termage::is_url("https://example.com/cat.png"));
/// assert!(!termage::is_url("cat.png"));
/// ```
pub fn is_url(input: &str) -> bool {
    let lower = input.get(..8).unwrap_or(input).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// The file extension (lower case, no dot) of a path or web address, ignoring
/// any `?query` or `#fragment`.
///
/// ```
/// assert_eq!(termage::extension("clip.MP4"), Some("mp4".to_string()));
/// assert_eq!(termage::extension("https://x.org/a/b.gif?size=2"), Some("gif".to_string()));
/// assert_eq!(termage::extension("https://x.org/"), None);
/// ```
pub fn extension(input: &str) -> Option<String> {
    let path = if is_url(input) {
        let end = input.find(['?', '#']).unwrap_or(input.len());
        let without_query = &input[..end];
        // drop "https://host", keeping only the path part
        let after_scheme = without_query.split_once("://")?.1;
        after_scheme.split_once('/').map(|(_, path)| path)?
    } else {
        input
    };
    Path::new(path)
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
}

/// Reads the bytes of a file, or downloads them if `input` is a web address.
pub fn read(input: &str) -> Result<Vec<u8>, Error> {
    if is_url(input) {
        download(input)
    } else {
        Ok(fs::read(input)?)
    }
}

#[cfg(feature = "net")]
fn download(url: &str) -> Result<Vec<u8>, Error> {
    let fail = |error: &dyn std::fmt::Display| Error::Download(format!("{error}"));
    let mut response = ureq::get(url).call().map_err(|error| fail(&error))?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD_BYTES)
        .read_to_vec()
        .map_err(|error| fail(&error))
}

#[cfg(not(feature = "net"))]
fn download(_url: &str) -> Result<Vec<u8>, Error> {
    Err(Error::Download(
        "this build of termage has no network support (the `net` feature)".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_recognised_whatever_the_case() {
        assert!(is_url("http://a.b/c.png"));
        assert!(is_url("HTTPS://A.B/C.PNG"));
        assert!(!is_url("ftp://a.b/c.png"));
        assert!(!is_url("./http.png"));
        assert!(!is_url(""));
        assert!(!is_url("http"));
    }

    #[test]
    fn extensions_ignore_queries_fragments_and_hosts() {
        assert_eq!(extension("a/b/pic.PNG"), Some("png".into()));
        assert_eq!(
            extension("https://x.org/v/clip.webm#t=5"),
            Some("webm".into())
        );
        assert_eq!(
            extension("https://example.com/pic.png?x=1.gif"),
            Some("png".into())
        );
        // a dot in the host name is not an extension
        assert_eq!(extension("https://example.com"), None);
        assert_eq!(extension("noextension"), None);
    }

    #[test]
    fn reading_a_missing_file_is_an_io_error() {
        assert!(matches!(read("not/here.png"), Err(Error::Io(_))));
    }

    #[test]
    fn an_unreachable_address_is_a_download_error() {
        // port 1 on localhost: nothing listens there
        let error = read("http://127.0.0.1:1/x.png").unwrap_err();
        assert!(matches!(error, Error::Download(_)), "{error:?}");
    }
}
