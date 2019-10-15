#!/bin/sh

RS=`cat security.txt`

curl -X POST \
	--header "Accept: application/json" \
	--header "Cookie: .ROBLOSECURITY=$RS" \
	-F "config=@config.json;filename=\"config.json\"" \
	-F "apple=@apple.png;filename=\"apple.png\"" \
	"https://publish.roblox.com/v1/assets/upload"