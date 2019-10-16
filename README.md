# Tarmac
Tarmac is a tool that manages assets for Roblox projects on the command line. It paves the way for hermetic place builds when used with tools like Rojo.

## Installation
Tarmac requires Rust 1.37+. When releases are built, pre-built binaries for Windows and macOS will be available.

```bash
cargo install --git https://github.com/rojo-rbx/tarmac.git
```

## Usage
Tarmac is still an early work in progress, but is starting to be useful. For full usage, use `tarmac --help`.

### Syncing a project
Tarmac can automatically discover and upload the images used in your project.

```bash
tarmac sync
```

Tarmac will upload any assets that have changed to Roblox.com.

It'll also create two files next to each asset:

* A `.tarmac.json` file, which contains the uploaded asset ID and a hash of the file's contents.
* A `.lua` file, which is a Roblox `ModuleScript` that can be imported to get the URL of the uploaded asset.

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