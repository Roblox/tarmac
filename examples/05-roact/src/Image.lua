local ReplicatedStorage = game:GetService("ReplicatedStorage")

local Roact = require(ReplicatedStorage.Modules.Roact)

local ResolutionScale = require(script.Parent.ResolutionScale)

local function join(...)
	local new = {}

	for i = 1, select("#", ...) do
		for key, value in pairs((select(i, ...))) do
			new[key] = value
		end
	end

	return new
end

local function applyImageProp(props, scale, imageSource)
	local imageType = typeof(imageSource)

	if imageType == "string" or imageType == "nil" then
		return join(props, { Image = imageSource })
	elseif imageType == "table" then
		return join(props, props.Image)
	elseif imageType == "function" then
		local afterScale = imageSource(scale)
		return applyImageProp(props, scale, afterScale)
	else
		error(string.format(
			"Unexpected type for prop 'Image'. Expected nil, string, table, or function, but got %s",
			imageType))
	end
end

local function createImageComponent(hostComponent)
	return function(props)
		return ResolutionScale.with(function(scale)
			local fullProps = applyImageProp(props, scale, props.Image)
			return Roact.createElement(hostComponent, fullProps)
		end)
	end
end

return {
	Label = createImageComponent("ImageLabel"),
	Button = createImageComponent("ImageButton"),
}