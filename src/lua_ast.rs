//! Defines part of a Lua AST, used for generating human-readable code in a
//! composable way for Tarmac.
//!
//! Eventually, it might be a good idea to replace this module with something
//! like full-moon (https://github.com/Kampfkarren/full-moon) or another real
//! Lua AST library.

use std::fmt::{self, Write};

trait FmtLua {
    fn fmt_lua(&self, output: &mut LuaStream<'_>) -> fmt::Result;

    fn fmt_table_key(&self, output: &mut LuaStream<'_>) -> fmt::Result {
        write!(output, "[")?;
        self.fmt_lua(output)?;
        write!(output, "]")
    }
}

macro_rules! proxy_display {
    ( $target: ty ) => {
        impl fmt::Display for $target {
            fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
                let mut stream = LuaStream::new(output);
                FmtLua::fmt_lua(self, &mut stream)
            }
        }
    };
}

pub(crate) struct Block {
    pub statements: Vec<Statement>,
}

impl FmtLua for Block {
    fn fmt_lua(&self, output: &mut LuaStream<'_>) -> fmt::Result {
        for statement in &self.statements {
            statement.fmt_lua(output)?;
            writeln!(output)?;
        }

        Ok(())
    }
}

proxy_display!(Block);

pub(crate) enum Statement {
    Return(Literal),
}

impl FmtLua for Statement {
    fn fmt_lua(&self, output: &mut LuaStream<'_>) -> fmt::Result {
        match self {
            Self::Return(literal) => {
                write!(output, "return ")?;
                literal.fmt_lua(output)
            }
        }
    }
}

pub(crate) enum Literal {
    String(String),
    Table(TableLiteral),
}

impl FmtLua for Literal {
    fn fmt_lua(&self, output: &mut LuaStream<'_>) -> fmt::Result {
        match self {
            Self::Table(inner) => inner.fmt_lua(output),
            Self::String(inner) => inner.fmt_lua(output),
        }
    }

    fn fmt_table_key(&self, output: &mut LuaStream<'_>) -> fmt::Result {
        match self {
            Self::Table(inner) => inner.fmt_table_key(output),
            Self::String(inner) => inner.fmt_table_key(output),
        }
    }
}

impl From<String> for Literal {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&'_ str> for Literal {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<TableLiteral> for Literal {
    fn from(value: TableLiteral) -> Self {
        Self::Table(value)
    }
}

impl FmtLua for String {
    fn fmt_lua(&self, output: &mut LuaStream<'_>) -> fmt::Result {
        write!(output, "\"{}\"", self)
    }

    fn fmt_table_key(&self, output: &mut LuaStream<'_>) -> fmt::Result {
        if is_valid_ident(self) {
            write!(output, "{}", self)
        } else {
            write!(output, "[\"{}\"]", self)
        }
    }
}

pub(crate) struct TableLiteral {
    pub entries: Vec<(Literal, Literal)>,
}

impl FmtLua for TableLiteral {
    fn fmt_lua(&self, output: &mut LuaStream<'_>) -> fmt::Result {
        writeln!(output, "{{")?;
        output.indent();

        for (key, value) in &self.entries {
            key.fmt_table_key(output)?;
            write!(output, " = ")?;
            value.fmt_lua(output)?;
            writeln!(output, ",")?;
        }

        output.unindent();
        write!(output, "}}")
    }
}

/// Tells whether the given string is a valid Lua identifier.
fn is_valid_ident(value: &str) -> bool {
    let mut chars = value.chars();

    match chars.next() {
        Some(first) => {
            if !first.is_alphabetic() {
                return false;
            }
        }
        None => return false,
    }

    chars.all(|x| x.is_ascii_alphanumeric())
}

struct LuaStream<'a> {
    indent_level: usize,
    is_start_of_line: bool,
    inner: &'a mut (dyn fmt::Write + 'a),
}

impl fmt::Write for LuaStream<'_> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let mut is_first_line = true;

        for line in value.split('\n') {
            if is_first_line {
                is_first_line = false;
            } else {
                self.line()?;
            }

            if !line.is_empty() {
                if self.is_start_of_line {
                    self.is_start_of_line = false;
                    let indentation = "\t".repeat(self.indent_level);
                    self.inner.write_str(&indentation)?;
                }

                self.inner.write_str(line)?;
            }
        }

        Ok(())
    }
}

impl<'a> LuaStream<'a> {
    fn new(inner: &'a mut (dyn fmt::Write + 'a)) -> Self {
        LuaStream {
            indent_level: 0,
            is_start_of_line: true,
            inner,
        }
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn unindent(&mut self) {
        assert!(self.indent_level > 0);
        self.indent_level -= 1;
    }

    fn line(&mut self) -> fmt::Result {
        self.is_start_of_line = true;
        self.inner.write_str("\n")
    }
}
