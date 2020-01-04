# Tarmac
Tarmac is a tool that manages assets for Roblox projects on the command line. It paves the way for hermetic place builds when used with tools like Rojo.

## Installation from Crates.io
Tarmac requires Rust 1.37+.

```bash
cargo install tarmac
```

## Usage
Tarmac is still an early work in progress, but is starting to be useful. For full usage, use `tarmac --help`.

### Syncing a project
Tarmac can automatically discover and upload the images used in your project.

```bash
tarmac sync --target roblox
```

Tarmac will upload any assets that have changed to Roblox.com.

It'll also create a central manifest file named `tarmac-manifest.toml`. It has all the files it found, their asset IDs, and the hash of their contents. This manifest can be processed by other tools to update assets from model files, Rojo projects, and more, but currently isn't used by anything besides Tarmac itself.

Tarmac can also optionally generate code to make importing images from Lua code more convenient. To do that, make a `tarmac.toml` file in your project:

```toml
[default]
# Options are 'none' (default), 'asset-url', and 'slice'
codegen = "asset-url"
```

Run Tarmac again and it'll create Lua files that look like this:

```lua
return "rbxassetid://12345678"
```

These files will be turned into `ModuleScript` objects by a tool like Rojo and make it incredibly easy to use assets in your code:

```lua
local ImageA = require(Assets.A)

local decal = Instance.new("Decal")
decal.Texture = ImageA
```

### Upload an Image
```bash
tarmac upload-image image.png --name "My Cool Image"
```

Tarmac will print the ID of the uploaded image to stdout, and any status messages to stderr. The output of this command can be turned into an asset URL as `rbxassetid://RETURNED_ID_HERE`.

## Vision
- Tool to crawl tree for asset files (`tarmac`)
	- Upload and copy assets
		- Assets will be in the `content` folder during development
		- Assets will be uploaded to Roblox.com when deploying
	- Produce asset manifests detailing what the status of each asset is
- Rojo user plugin (`tarmac-rojo`)
	- Reads asset manifest to figure out mapping
	- Maps IDs and file paths to assets uploaded by tool

## License
Tarmac is available under the MIT license. See [LICENSE.txt](LICENSE.txt) for details.