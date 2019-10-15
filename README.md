# Tarmac
Tarmac paves the way to hermetic place builds when used with tools like Rojo.

## Usage
Tarmac is still early, but is already starting to be useful. For full usage, use `tarmac --help`.

### Upload an Image
```bash
tarmac upload-image image.png --name "My Cool Image"
```

Tarmac will print the ID of the uploaded image to stdout, and any status messages to stderr. The output of this command can be turned into an asset URL as `rbxassetid://RETURNED_ID_HERE`.

## Vision
- Tool to crawl tree for asset files (`tarmac`)
	- Upload, copy, etc, assets into place they're being deployed
		- Content folder when local
		- Roblox website when deploying to production
	- Produce asset manifest in root of project
- Rojo user plugin (`tarmac-rojo`)
	- Reads asset manifest to figure out mapping
	- Maps IDs and file paths to assets uploaded by tool

## License
Tarmac is available under the MIT license. See [LICENSE.txt](LICENSE.txt) for details.