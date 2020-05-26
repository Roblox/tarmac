local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Players = game:GetService("Players")

local Roact = require(ReplicatedStorage.Modules.Roact)

local Assets = require(script.Assets)
local Image = require(script.Image)
local ResolutionScale = require(script.ResolutionScale)

local actualImageSize = Vector2.new(36, 36)
local displayedSize = actualImageSize * 8

local ui = Roact.createElement("ScreenGui", nil, {
	ScaleProvider = Roact.createElement(ResolutionScale.CyclingProvider, nil, {
		Image = Roact.createElement(Image.Label, {
			Image = Assets.accept,
			Size = UDim2.fromOffset(displayedSize.X, displayedSize.Y),
			Position = UDim2.fromScale(0.5, 0.5),
			AnchorPoint = Vector2.new(0.5, 0.5),
			BackgroundColor3 = Color3.new(0, 0, 0),
			BorderSizePixel = 0,
		}),
	}),
})

Roact.mount(ui, Players.LocalPlayer.PlayerGui, "Tarmac and Roact Example")