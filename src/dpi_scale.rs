use std::path::Path;

use regex::Regex;

/// Given a path, checks if it's marked as being high DPI.
///
/// Examples of the convention Tarmac uses:
///
/// - foo.png (1x)
/// - foo@1x.png (1x)
/// - foo@2x.png (2x)
/// - foo@3x.png (3x)
pub(crate) fn dpi_scale_for_path<P: AsRef<Path>>(path: P) -> u32 {
    lazy_static::lazy_static! {
        static ref DPI_PATTERN: Regex = Regex::new(r"@(\d+)x\..+?$").unwrap();
    }

    let path = path.as_ref();

    let file_name = match path.file_name().unwrap().to_str() {
        Some(name) => name,

        // If the filename isn't valid Unicode, we'll assume it's a 1x asset.
        None => {
            log::warn!(
                "Path {} had invalid Unicode, considering it a 1x asset...",
                path.display()
            );

            return 1;
        }
    };

    match DPI_PATTERN.captures(file_name) {
        Some(captures) => {
            let scale_str = captures.get(1).unwrap().as_str();
            scale_str.parse().unwrap()
        }
        None => 1,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn no_attached_scale() {
        assert_eq!(dpi_scale_for_path("foo.png"), 1);
        assert_eq!(dpi_scale_for_path("foo.blah.png"), 1);
        assert_eq!(dpi_scale_for_path("foo/bar/baz/hello.png"), 1);
    }

    #[test]
    fn explicit_1x() {
        assert_eq!(dpi_scale_for_path("layerify@1x.png"), 1);
        assert_eq!(dpi_scale_for_path("layerify.blah@1x.png"), 1);
        assert_eq!(dpi_scale_for_path("layerify@1x.png.bak"), 1);
        assert_eq!(dpi_scale_for_path("some/path/to/image/nice@1x.png"), 1);
    }

    #[test]
    fn explicit_not_1x() {
        assert_eq!(dpi_scale_for_path("cool-company@2x.png"), 2);
        assert_eq!(dpi_scale_for_path("engineers@10x.png"), 10);
        assert_eq!(dpi_scale_for_path("we.like.dots@3x.png"), 3);
        assert_eq!(dpi_scale_for_path("backup-your-stuff@4x.png.bak"), 4);
    }
}
