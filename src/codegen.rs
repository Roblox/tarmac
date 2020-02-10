//! Defines how Tarmac generates Lua code for linking to assets.
//!
//! Tarmac uses structs with `Display` impls to build up templates.

use std::{collections::HashMap, fmt};

use crate::{asset_name::AssetName, data::ImageSlice};

const CODEGEN_HEADER: &str =
    "-- This file was @generated by Tarmac. It is not intended for manual editing.";

pub(crate) struct TestBatchTemplate {
    pub inputs: Vec<AssetName>,
}

impl fmt::Display for TestBatchTemplate {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        enum Item<'a> {
            Folder(HashMap<&'a str, Item<'a>>),
            Input(&'a AssetName),
        }

        let mut indexed_items: HashMap<&str, Item<'_>> = HashMap::new();

        for input in &self.inputs {
            let mut components = input.components().peekable();
            let mut current_dir = &mut indexed_items;

            loop {
                let name = components.next().unwrap();
                let has_more_components = components.peek().is_some();

                if has_more_components {
                    let next_entry = current_dir
                        .entry(name)
                        .or_insert_with(|| Item::Folder(HashMap::new()));

                    match next_entry {
                        Item::Folder(next_dir) => {
                            current_dir = next_dir;
                        }
                        Item::Input(_) => {
                            panic!("Malformed input tree");
                        }
                    }
                } else {
                    current_dir.insert(name, Item::Input(input));
                    break;
                }
            }
        }

        writeln!(formatter, "{}", CODEGEN_HEADER)?;
        write!(formatter, "return ")?;

        impl Item<'_> {
            fn pretty_fmt(
                &self,
                formatter: &mut fmt::Formatter,
                indent_level: usize,
            ) -> fmt::Result {
                match self {
                    Item::Folder(children) => {
                        let indentation = "\t".repeat(indent_level);
                        writeln!(formatter, "{{")?;

                        for (name, child) in children {
                            write!(formatter, "{}[\"{}\"] = ", indentation, name)?;
                            child.pretty_fmt(formatter, indent_level + 1)?;
                            writeln!(formatter, ",")?;
                        }

                        let indentation = "\t".repeat(indent_level - 1);
                        write!(formatter, "{}}}", indentation)?;
                    }
                    Item::Input(asset_name) => {
                        write!(formatter, "\"{}\"", asset_name)?;
                    }
                }

                Ok(())
            }
        }

        Item::Folder(indexed_items).pretty_fmt(formatter, 1)?;

        Ok(())
    }
}

/// Codegen template for CodegenKind::AssetUrl
pub(crate) struct AssetUrlTemplate {
    pub id: u64,
}

impl fmt::Display for AssetUrlTemplate {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(formatter, "{}", CODEGEN_HEADER)?;
        writeln!(formatter, "return \"rbxassetid://{}\"", self.id)?;

        Ok(())
    }
}

pub(crate) struct UrlAndSliceTemplate {
    pub id: u64,
    pub slice: Option<ImageSlice>,
}

impl fmt::Display for UrlAndSliceTemplate {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(formatter, "{}", CODEGEN_HEADER)?;

        writeln!(formatter, "return {{")?;
        writeln!(formatter, "\tImage = \"rbxassetid://{}\",", self.id)?;

        if let Some(slice) = self.slice {
            let offset = slice.min();
            let size = slice.size();

            writeln!(
                formatter,
                "\tImageRectOffset = Vector2.new({}, {}),",
                offset.0, offset.1
            )?;
            writeln!(
                formatter,
                "\tImageRectSize = Vector2.new({}, {}),",
                size.0, size.1
            )?;
        }

        writeln!(formatter, "}}")?;

        Ok(())
    }
}
