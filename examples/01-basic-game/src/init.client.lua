-- This isn't really a game, but shows how you might reference an image uploaded
-- by Tarmac.

local Players = game:GetService("Players")

-- See default.project.json for how Assets gets put into our game.
local Assets = script.Parent.Assets

-- Tarmac makes it so we can pretend we're importing our image files themselves!
-- Once Tarmac runs, there will be Lua files next to each image.
local logo = require(Assets.logo)

local gui = Instance.new("ScreenGui")
gui.Parent = Players.LocalPlayer.PlayerGui

local gameplay = Instance.new("ImageButton")
gameplay.Size = UDim2.new(0, 256, 0, 256)
gameplay.Position = UDim2.new(0.5, 0, 0.5, 0)
gameplay.AnchorPoint = Vector2.new(0.5, 0.5)
gameplay.Image = logo
gameplay.Parent = gui