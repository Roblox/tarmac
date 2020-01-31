# Tarmac Changelog

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