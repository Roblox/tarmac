# Tarmac
Tarmac paves the way to hermetic place builds when used with tools like Rojo.

- Tool to crawl tree for asset files
	- Upload, copy, etc, assets into place they're being deployed
		- Content folder when local
		- Roblox website when deploying to production
	- Produce asset manifest in root of project
- Rojo user plugin
	- Reads asset manifest to figure out mapping
	- Maps IDs and file paths to assets uploaded by tool