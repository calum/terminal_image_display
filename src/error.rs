//! The error type for everything that can go wrong while showing an image.

use std::fmt;
use std::io;

/// Why an image could not be loaded or shown.
#[derive(Debug)]
pub enum Error {
    /// The file could not be opened, or the terminal could not be written to.
    Io(io::Error),
    /// The file is not an image termage can decode.
    Image(image::ImageError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(error) => write!(f, "{error}"),
            Error::Image(error) => write!(f, "could not decode the image: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(error) => Some(error),
            Error::Image(error) => Some(error),
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::Io(error)
    }
}

impl From<image::ImageError> for Error {
    fn from(error: image::ImageError) -> Error {
        match error {
            image::ImageError::IoError(error) => Error::Io(error),
            error => Error::Image(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_errors_inside_image_errors_are_unwrapped() {
        let error = Error::from(image::ImageError::IoError(io::Error::new(
            io::ErrorKind::NotFound,
            "no such file",
        )));
        assert!(matches!(error, Error::Io(_)));
        assert_eq!(error.to_string(), "no such file");
    }

    #[test]
    fn decode_errors_say_what_failed() {
        let error = Error::from(image::ImageError::Limits(
            image::error::LimitError::from_kind(image::error::LimitErrorKind::DimensionError),
        ));
        assert!(error.to_string().starts_with("could not decode the image"));
        assert!(std::error::Error::source(&error).is_some());
    }
}
