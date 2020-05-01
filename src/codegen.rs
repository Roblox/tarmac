//! Defines how Tarmac generates Lua code for linking to assets.
//!
//! Tarmac uses a small Lua AST to build up generated code.

use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::{self, Path},
};

use fs_err::File;

use crate::{
    data::ImageSlice,
    data::SyncInput,
    lua_ast::{Block, Expression, Function, IfBlock, Statement, Table},
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

/// Tree used to track and group inputs hierarchically, before turning them into
/// Lua tables.
enum GroupedItem<'a> {
    Folder {
        children_by_name: BTreeMap<String, GroupedItem<'a>>,
    },
    InputGroup {
        inputs_by_dpi_scale: BTreeMap<u32, &'a SyncInput>,
    },
}

/// Perform codegen for a group of inputs who have `codegen_path` defined.
///
/// We'll build up a Lua file containing nested tables that match the structure
/// of the input's path with its base path stripped away.
fn codegen_grouped(output_path: &Path, inputs: &[&SyncInput]) -> io::Result<()> {
    let mut root_folder: BTreeMap<String, GroupedItem<'_>> = BTreeMap::new();

    // First, collect all of the inputs and group them together into a tree
    // according to their relative paths.
    for &input in inputs {
        // Not all inputs will be marked for codegen. We can eliminate those
        // right away.
        if !input.config.codegen {
            continue;
        }

        // If we can't construct a relative path, there isn't a sensible name
        // that we can use to refer to this input.
        let relative_path = input
            .path
            .strip_prefix(&input.config.base_path)
            .expect("Input base path was not a base path for input");

        // Collapse `..` path segments so that we can map this path onto our
        // tree of inputs.
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

        // Navigate down the tree, creating any folder entries that don't exist
        // yet.
        let mut current_dir = &mut root_folder;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                // We assume that the last segment of a path must be a file.

                let name = &input.stem_name;

                let input_group = match current_dir.get_mut(name) {
                    Some(existing) => existing,
                    None => {
                        let input_group = GroupedItem::InputGroup {
                            inputs_by_dpi_scale: BTreeMap::new(),
                        };
                        current_dir.insert(name.to_owned(), input_group);
                        current_dir.get_mut(name).unwrap()
                    }
                };

                if let GroupedItem::InputGroup {
                    inputs_by_dpi_scale,
                } = input_group
                {
                    inputs_by_dpi_scale.insert(input.dpi_scale, input);
                } else {
                    unreachable!();
                }
            } else {
                let name = segment.to_str().unwrap().to_owned();
                let next_entry = current_dir
                    .entry(name)
                    .or_insert_with(|| GroupedItem::Folder {
                        children_by_name: BTreeMap::new(),
                    });

                if let GroupedItem::Folder { children_by_name } = next_entry {
                    current_dir = children_by_name;
                } else {
                    unreachable!();
                }
            }
        }
    }

    fn build_item(item: &GroupedItem<'_>) -> Option<Expression> {
        match item {
            GroupedItem::Folder { children_by_name } => {
                let entries = children_by_name
                    .iter()
                    .filter_map(|(name, child)| build_item(child).map(|item| (name.into(), item)))
                    .collect();

                Some(Expression::table(entries))
            }
            GroupedItem::InputGroup {
                inputs_by_dpi_scale,
            } => {
                if inputs_by_dpi_scale.len() == 1 {
                    // If there is exactly one input in this group, we can
                    // generate code knowing that there are no high DPI variants
                    // to choose from.

                    let input = inputs_by_dpi_scale.values().next().unwrap();

                    if let Some(id) = input.id {
                        if let Some(slice) = input.slice {
                            Some(codegen_url_and_slice(id, slice))
                        } else {
                            Some(codegen_just_asset_url(id))
                        }
                    } else {
                        None
                    }
                } else {
                    // In this case, we have the same asset in multiple
                    // different DPI scales. We can generate code to pick
                    // between them at runtime.
                    Some(codegen_with_high_dpi_options(inputs_by_dpi_scale))
                }
            }
        }
    }

    let root_item = build_item(&GroupedItem::Folder {
        children_by_name: root_folder,
    })
    .unwrap();
    let ast = Statement::Return(root_item);

    let mut file = File::create(output_path)?;
    writeln!(file, "{}", CODEGEN_HEADER)?;
    write!(file, "{}", ast)?;

    Ok(())
}

/// Perform codegen for a group of inputs that don't have `codegen_path`
/// defined, and so generate individual files.
fn codegen_individual(inputs: &[&SyncInput]) -> io::Result<()> {
    for input in inputs {
        let maybe_expression = if input.config.codegen {
            if let Some(id) = input.id {
                if let Some(slice) = input.slice {
                    Some(codegen_url_and_slice(id, slice))
                } else {
                    Some(codegen_just_asset_url(id))
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(expression) = maybe_expression {
            let ast = Statement::Return(expression);

            let path = input.path.with_extension("lua");

            let mut file = File::create(path)?;
            writeln!(file, "{}", CODEGEN_HEADER)?;
            write!(file, "{}", ast)?;
        }
    }

    Ok(())
}

fn codegen_url_and_slice(id: u64, slice: ImageSlice) -> Expression {
    let offset = slice.min();
    let size = slice.size();

    let mut table = Table::new();
    table.add_entry("Image", format!("rbxassetid://{}", id));
    table.add_entry(
        "ImageRectOffset",
        Expression::Raw(format!("Vector2.new({}, {})", offset.0, offset.1)),
    );

    table.add_entry(
        "ImageRectSize",
        Expression::Raw(format!("Vector2.new({}, {})", size.0, size.1)),
    );

    Expression::Table(table)
}

fn codegen_just_asset_url(id: u64) -> Expression {
    Expression::String(format!("rbxassetid://{}", id))
}

fn codegen_dpi_option(input: &SyncInput) -> (Expression, Block) {
    let condition = Expression::Raw(format!("dpiScale >= {}", input.dpi_scale));

    // FIXME: We should probably pull data out of SyncInput at the start of
    // codegen so that we can handle invariants like this.
    let id = input.id.unwrap();

    let value = match input.slice {
        Some(slice) => codegen_url_and_slice(id, slice),
        None => codegen_just_asset_url(id),
    };

    let body = Statement::Return(value);

    (condition, body.into())
}

fn codegen_with_high_dpi_options(inputs: &BTreeMap<u32, &SyncInput>) -> Expression {
    let args = "dpiScale".to_owned();

    let mut options_high_to_low = inputs.values().rev().peekable();

    let highest_dpi_option = options_high_to_low.next().unwrap();
    let (highest_cond, highest_body) = codegen_dpi_option(highest_dpi_option);

    let mut if_block = IfBlock::new(highest_cond, highest_body);

    while let Some(dpi_option) = options_high_to_low.next() {
        let (cond, body) = codegen_dpi_option(dpi_option);

        if options_high_to_low.peek().is_some() {
            if_block.else_if_blocks.push((cond, body));
        } else {
            if_block.else_block = Some(body);
        }
    }

    let statements = vec![Statement::If(if_block)];

    Expression::Function(Function::new(args, statements))
}
