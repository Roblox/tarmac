# Tarmac Changelog

## 0.3.1 (2020-02-04)
* Fixed `tarmac sync` sometimes re-uploading more images than it should. ([#19](https://github.com/rojo-rbx/tarmac/pull/19))
* Updated `tarmac-manifest.toml` to require hashes. This might cause errors when upgrading to 0.3.1; they can be fixed by deleting your manifest and syncing again.

## 0.3.0 (2020-01-31)
* Rewrote texture packing routine with a new library, [Packos](https://crates.io/crates/packos).
	* This should fix textures overlapping eachother when running Tarmac with automatic spritesheets enabled.

## 0.2.0 (2020-01-21)
* Revamped configuration format.
* Added support for automatically packing spritesheets.
* Added support for nesting projects inside eachother via `include`.
* Added support for grabbing inputs by glob pattern.

## 0.1.0 (2020-01-03)
* Initial release.