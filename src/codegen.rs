//! Defines how Tarmac generates Lua code for linking to assets.
//!
//! Tarmac uses a small Lua AST to build up generated code.

use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::{self, Path},
};

use crate::{
    data::SyncInput,
    data::{CodegenKind, ImageSlice},
    fs::File,
    lua_ast::{Expression, Statement, Table},
};

const CODEGEN_HEADER: &str =
    "-- This file was @generated by Tarmac. It is not intended for manual editing.";

pub fn perform_codegen(output_path: Option<&Path>, inputs: &[&SyncInput]) -> io::Result<()> {
    if let Some(path) = output_path {
        codegen_grouped(path, inputs)
    } else {
        codegen_individual(inputs)
    }
}

fn codegen_grouped(output_path: &Path, inputs: &[&SyncInput]) -> io::Result<()> {
    enum Item<'a> {
        Folder(BTreeMap<&'a str, Item<'a>>),
        Input(&'a SyncInput),
    }

    let mut indexed_items: BTreeMap<&str, Item<'_>> = BTreeMap::new();

    for input in inputs {
        let relative_path = input
            .path
            .strip_prefix(&input.config.base_path)
            .expect("Input base path was not a base path for input");

        let mut segments = Vec::new();
        for component in relative_path.components() {
            match component {
                path::Component::Prefix(_)
                | path::Component::RootDir
                | path::Component::Normal(_) => segments.push(Path::new(component.as_os_str())),
                path::Component::CurDir => {}
                path::Component::ParentDir => assert!(segments.pop().is_some()),
            }
        }

        let mut current_dir = &mut indexed_items;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                let name = segment.file_stem().unwrap().to_str().unwrap();
                current_dir.insert(name, Item::Input(input));
            } else {
                let name = segment.to_str().unwrap();
                let next_entry = current_dir
                    .entry(name)
                    .or_insert_with(|| Item::Folder(BTreeMap::new()));

                match next_entry {
                    Item::Folder(next_dir) => {
                        current_dir = next_dir;
                    }
                    Item::Input(_) => {
                        panic!("Malformed input tree");
                    }
                }
            }
        }
    }

    fn build_item(item: &Item<'_>) -> Option<Expression> {
        match item {
            Item::Folder(children) => {
                let entries = children
                    .iter()
                    .filter_map(|(&name, child)| build_item(child).map(|item| (name.into(), item)))
                    .collect();

                Some(Expression::table(entries))
            }
            Item::Input(input) => match input.config.codegen {
                Some(CodegenKind::AssetUrl) => {
                    if let Some(id) = input.id {
                        let template = AssetUrlTemplate { id };

                        Some(template.to_lua())
                    } else {
                        None
                    }
                }
                Some(CodegenKind::UrlAndSlice) => {
                    if let Some(id) = input.id {
                        let template = UrlAndSliceTemplate {
                            id,
                            slice: input.slice,
                        };

                        Some(template.to_lua())
                    } else {
                        None
                    }
                }
                None => None,
            },
        }
    }

    let root_item = build_item(&Item::Folder(indexed_items)).unwrap();
    let ast = Statement::Return(root_item);

    let mut file = File::create(output_path)?;
    writeln!(file, "{}", CODEGEN_HEADER)?;
    write!(file, "{}", ast)?;

    Ok(())
}

fn codegen_individual(inputs: &[&SyncInput]) -> io::Result<()> {
    for input in inputs {
        if let Some(codegen) = input.config.codegen {
            let maybe_expression = match codegen {
                CodegenKind::AssetUrl => {
                    if let Some(id) = input.id {
                        let template = AssetUrlTemplate { id };

                        Some(template.to_lua())
                    } else {
                        None
                    }
                }

                CodegenKind::UrlAndSlice => {
                    if let Some(id) = input.id {
                        let template = UrlAndSliceTemplate {
                            id,
                            slice: input.slice,
                        };

                        Some(template.to_lua())
                    } else {
                        None
                    }
                }
            };

            if let Some(expression) = maybe_expression {
                let ast = Statement::Return(expression);

                let path = input.path.with_extension("lua");

                let mut file = File::create(path)?;
                writeln!(file, "{}", CODEGEN_HEADER)?;
                write!(file, "{}", ast)?;
            }
        }
    }

    Ok(())
}

/// Codegen template for CodegenKind::AssetUrl
pub(crate) struct AssetUrlTemplate {
    pub id: u64,
}

impl AssetUrlTemplate {
    fn to_lua(&self) -> Expression {
        Expression::String(format!("rbxassetid://{}", self.id))
    }
}

pub(crate) struct UrlAndSliceTemplate {
    pub id: u64,
    pub slice: Option<ImageSlice>,
}

impl UrlAndSliceTemplate {
    fn to_lua(&self) -> Expression {
        let mut table = Table::new();

        table.add_entry("Image", format!("rbxassetid://{}", self.id));

        if let Some(slice) = self.slice {
            let offset = slice.min();
            let size = slice.size();

            table.add_entry(
                "ImageRectOffset",
                Expression::Raw(format!("Vector2.new({}, {})", offset.0, offset.1)),
            );

            table.add_entry(
                "ImageRectSize",
                Expression::Raw(format!("Vector2.new({}, {})", size.0, size.1)),
            );
        }

        Expression::Table(table)
    }
}
