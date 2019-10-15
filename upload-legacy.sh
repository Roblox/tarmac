#!/bin/sh

RS=`cat security.txt`

curl -X POST \
	--header "Cookie: .ROBLOSECURITY=$RS" \
	--data "@apple.png" \
	"https://data.roblox.com/Data/Upload.ashx?assetid=0&type=Decal&name=Apple&description=MyDescription"