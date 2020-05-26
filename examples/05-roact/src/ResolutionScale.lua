local GuiService = game:GetService("GuiService")
local ReplicatedStorage = game:GetService("ReplicatedStorage")

local Roact = require(ReplicatedStorage.Modules.Roact)

local Context = Roact.createContext()

-- CoreProvider accesses the engine's resolution scale from a core script only
-- API on GuiService.
local CoreProvider = Roact.Component:extend("ResolutionScale.CoreProvider")

function CoreProvider:init()
	local success, scale = pcall(function()
		return GuiService:GetResolutionScale()
	end)

	if success then
		self.scale = scale
	else
		self.scale = 1
		warn("Resolution scale could not be determined as GuiService:GetResolutionScale() failed. Falling back to 1x scaling.")
	end
end

function CoreProvider:render()
	return Roact.createElement(Context.Provider, {
		value = self.scale,
	}, self.props[Roact.Children])
end

-- CyclingProvider is a debugging aid that cycles through 1x, 2x, and 3x image
-- scales every second.
local CyclingProvider = Roact.Component:extend("ResolutionScale.CyclingProvider")

function CyclingProvider:init()
	self:setState({
		scale = 1,
	})
end

function CyclingProvider:render()
	return Roact.createElement(Context.Provider, {
		value = self.state.scale,
	}, self.props[Roact.Children])
end

function CyclingProvider:didMount()
	self.isMounted = true

	delay(1, function()
		while self.isMounted do
			self:setState(function(state)
				return {
					scale = 1 + (state.scale % 3),
				}
			end)

			wait(1)
		end
	end)
end

function CyclingProvider:willUnmount()
	self.isMounted = false
end

local function withResolutionScale(callback)
	return Roact.createElement(Context.Consumer, {
		render = function(maybeScale)
			return callback(maybeScale or 1)
		end,
	})
end

return {
	Provider = Context.Provider,
	CoreProvider = Context.CoreProvider,
	CyclingProvider = CyclingProvider,
	with = withResolutionScale,
}