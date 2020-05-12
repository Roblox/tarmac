use std::path::{Path, PathBuf};

use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DpiAwarePathInfo {
    pub(crate) path_without_dpi_scale: PathBuf,
    pub(crate) dpi_scale: u32,
}

impl DpiAwarePathInfo {
    #[cfg(test)]
    fn new(path_without_dpi_scale: &str, dpi_scale: u32) -> Self {
        let path_without_dpi_scale = PathBuf::from(path_without_dpi_scale);

        Self {
            path_without_dpi_scale,
            dpi_scale,
        }
    }
}

/// Given a path, extracts its intended DPI scale and constructs a path without
/// DPI scale information in it. This can be used to group together multiple
/// versions of the same image.
pub(crate) fn extract_path_info<P: AsRef<Path>>(path: P) -> DpiAwarePathInfo {
    lazy_static::lazy_static! {
        static ref DPI_PATTERN: Regex = Regex::new(r"^(.+?)@(\d+)x(.+?)$").unwrap();
    }

    let path = path.as_ref();

    let file_name = match path.file_name().unwrap().to_str() {
        Some(name) => name,

        // If the filename isn't valid Unicode, this is an error.
        None => {
            panic!("Path {} had invalid Unicode", path.display());
        }
    };

    match DPI_PATTERN.captures(file_name) {
        Some(captures) => {
            let file_stem = captures.get(1).unwrap().as_str().to_owned();
            let scale_str = captures.get(2).unwrap().as_str();
            let suffix = captures.get(3).unwrap().as_str();
            let dpi_scale = scale_str.parse().unwrap();

            let file_name_without_dpi_scale = format!("{}{}", file_stem, suffix);
            let path_without_dpi_scale = path.with_file_name(&file_name_without_dpi_scale);

            DpiAwarePathInfo {
                path_without_dpi_scale,
                dpi_scale,
            }
        }
        None => DpiAwarePathInfo {
            path_without_dpi_scale: path.to_owned(),
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
            DpiAwarePathInfo::new("foo.png", 1)
        );

        assert_eq!(
            extract_path_info("foo.blah.png"),
            DpiAwarePathInfo::new("foo.blah.png", 1)
        );

        assert_eq!(
            extract_path_info("foo/bar/baz/hello.png"),
            DpiAwarePathInfo::new("foo/bar/baz/hello.png", 1)
        );
    }

    #[test]
    fn explicit_1x() {
        assert_eq!(
            extract_path_info("layerify@1x.png"),
            DpiAwarePathInfo::new("layerify.png", 1)
        );

        assert_eq!(
            extract_path_info("layerify.blah@1x.png"),
            DpiAwarePathInfo::new("layerify.blah.png", 1)
        );

        assert_eq!(
            extract_path_info("layerify@1x.png.bak"),
            DpiAwarePathInfo::new("layerify.png.bak", 1)
        );

        assert_eq!(
            extract_path_info("some/path/to/image/nice@1x.png"),
            DpiAwarePathInfo::new("some/path/to/image/nice.png", 1)
        );
    }

    #[test]
    fn explicit_not_1x() {
        assert_eq!(
            extract_path_info("cool-company@2x.png"),
            DpiAwarePathInfo::new("cool-company.png", 2)
        );

        assert_eq!(
            extract_path_info("engineers@10x.png"),
            DpiAwarePathInfo::new("engineers.png", 10)
        );

        assert_eq!(
            extract_path_info("we.like.dots@3x.png"),
            DpiAwarePathInfo::new("we.like.dots.png", 3)
        );

        assert_eq!(
            extract_path_info("backup-your-stuff@4x.png.bak"),
            DpiAwarePathInfo::new("backup-your-stuff.png.bak", 4)
        );
    }
}
