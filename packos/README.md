<div align="center">
    <h1>Packos</h1>
</div>

<div align="center">
    <a href="https://github.com/Roblox/tarmac/actions">
        <img src="https://github.com/Roblox/tarmac/workflows/CI/badge.svg" alt="GitHub Actions status" />
    </a>
    <a href="https://crates.io/crates/packos">
        <img src="https://img.shields.io/crates/v/packos.svg?label=latest%20release" alt="Latest release" />
    </a>
    <a href="https://docs.rs/packos">
        <img src="https://img.shields.io/badge/docs-docs.rs-orange.svg" alt="Packos docs on docs.rs" />
    </a>
</div>

<hr />

Packos is a small library for packing rectangles. It was built for [Tarmac](https://github.com/Roblox/tarmac), a tool that manages assets for Roblox projects, including packing images into spritesheets.

It's designed to:

1. Err on the side of simplicity.
2. Fit hard constraints:
	- Fixed padding
	- Minimum and maximum bucket sizes
	- Power-of-two dimensions

Packos leaves applying the rectangle packing solution to the consuming application.

## License
Packos is available under the MIT license. See [LICENSE.txt](LICENSE.txt) for details.