use std::path::Path;

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DpiAwarePathInfo {
    pub(crate) file_stem: String,
    pub(crate) dpi_scale: u32,
}

impl DpiAwarePathInfo {
    #[cfg(test)]
    fn new(file_stem: &str, dpi_scale: u32) -> Self {
        Self {
            file_stem: file_stem.to_owned(),
            dpi_scale,
        }
    }
}

/// Given a path, extracts its file stem and DPI scale.
///
/// If a DPI scale is found as part of the file name, is it removed from the
/// file stem.
pub(crate) fn extract_path_info<P: AsRef<Path>>(path: P) -> DpiAwarePathInfo {
    lazy_static::lazy_static! {
        static ref DPI_PATTERN: Regex = Regex::new(r"^(.+?)@(\d+)x$").unwrap();
    }

    let path = path.as_ref();

    let file_stem = match path.file_stem().unwrap().to_str() {
        Some(name) => name,

        // If the filename isn't valid Unicode, this is an error.
        None => {
            panic!("Path {} had invalid Unicode", path.display());
        }
    };

    match DPI_PATTERN.captures(file_stem) {
        Some(captures) => {
            let file_stem = captures.get(1).unwrap().as_str().to_owned();
            let scale_str = captures.get(2).unwrap().as_str();
            let dpi_scale = scale_str.parse().unwrap();

            DpiAwarePathInfo {
                file_stem,
                dpi_scale,
            }
        }
        None => DpiAwarePathInfo {
            file_stem: file_stem.to_owned(),
            dpi_scale: 1,
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn no_attached_scale() {
        assert_eq!(
            extract_path_info("foo.png"),
            Some(DpiAwarePathInfo::new("foo", 1))
        );

        assert_eq!(
            extract_path_info("foo.blah.png"),
            Some(DpiAwarePathInfo::new("foo.blah", 1))
        );

        assert_eq!(
            extract_path_info("foo/bar/baz/hello.png"),
            Some(DpiAwarePathInfo::new("hello", 1))
        );
    }

    #[test]
    fn explicit_1x() {
        assert_eq!(
            extract_path_info("layerify@1x.png"),
            Some(DpiAwarePathInfo::new("layerify", 1))
        );

        assert_eq!(
            extract_path_info("layerify.blah@1x.png"),
            Some(DpiAwarePathInfo::new("layerify.blah", 1))
        );

        assert_eq!(
            extract_path_info("layerify@1x.png.bak"),
            Some(DpiAwarePathInfo::new("layerify@1x.png", 1)),
        );

        assert_eq!(
            extract_path_info("some/path/to/image/nice@1x.png"),
            Some(DpiAwarePathInfo::new("nice", 1))
        );
    }

    #[test]
    fn explicit_not_1x() {
        assert_eq!(
            extract_path_info("cool-company@2x.png"),
            Some(DpiAwarePathInfo::new("cool-company", 2))
        );

        assert_eq!(
            extract_path_info("engineers@10x.png"),
            Some(DpiAwarePathInfo::new("engineers", 10))
        );

        assert_eq!(
            extract_path_info("we.like.dots@3x.png"),
            Some(DpiAwarePathInfo::new("we.like.dots", 3))
        );

        assert_eq!(
            extract_path_info("backup-your-stuff@4x.png.bak"),
            Some(DpiAwarePathInfo::new("backup-your-stuff@4x.png", 1))
        );
    }
}
