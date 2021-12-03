use crate::merge::ColorSet;
use crate::Metavariable;
use std::io::Write;

type Result = std::io::Result<()>;

pub trait TreeFormatter {
    type Output: std::io::Write;
    fn output(&mut self) -> &mut Self::Output;

    fn write_token(&mut self, tok: &[u8]) -> Result {
        self.output().write_all(tok)
    }

    fn write_metavariable(&mut self, mv: Metavariable) -> Result {
        write!(self.output(), "${}", mv.0)
    }

    fn write_colored(
        &mut self,
        _colors: ColorSet,
        write_tree: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        write_tree(self)
    }

    fn write_unchanged(&mut self) -> Result {
        write!(self.output(), "·")
    }

    fn write_changed(
        &mut self,
        write_del: impl FnOnce(&mut Self) -> Result,
        write_ins: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        write!(self.output(), "CHANGED![«")?;
        write_del(self)?;
        write!(self.output(), "» -> «")?;
        write_ins(self)?;
        write!(self.output(), "»]")
    }

    fn write_deleted(&mut self, write_del: impl FnOnce(&mut Self) -> Result) -> Result {
        write!(self.output(), "DELETED![")?;
        write_del(self)?;
        write!(self.output(), "]")
    }

    fn write_inserted(&mut self, write_ins: impl FnOnce(&mut Self) -> Result) -> Result {
        write!(self.output(), "INSERTED![")?;
        write_ins(self)?;
        write!(self.output(), "]")
    }

    fn write_mv_conflict(
        &mut self,
        mv: Metavariable,
        write_del: impl FnOnce(&mut Self) -> Result,
        write_repl: Option<impl FnOnce(&mut Self) -> Result>,
    ) -> Result {
        write!(self.output(), "MV_CONFLICT![${}: «", mv.0)?;
        write_del(self)?;
        write!(self.output(), "»")?;
        if let Some(write_repl) = write_repl {
            write!(self.output(), " <- «")?;
            write_repl(self)?;
            write!(self.output(), "»")?;
        }
        write!(self.output(), "]")
    }

    fn write_ins_conflict(
        &mut self,
        write_confl_iter: impl Iterator<Item = impl FnOnce(&mut Self) -> Result>,
    ) -> Result {
        write!(self.output(), "CONFLICT![")?;
        for (i, write_ins) in write_confl_iter.enumerate() {
            if i == 0 {
                write!(self.output(), "«")?
            } else {
                write!(self.output(), ", «")?
            }
            write_ins(self)?;
            write!(self.output(), "»")?;
        }
        write!(self.output(), "]")
    }

    fn write_del_conflict(
        &mut self,
        write_del: Option<impl FnOnce(&mut Self) -> Result>,
        write_ins: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        write!(self.output(), "DELETE_CONFLICT![«")?;
        if let Some(write_del) = write_del {
            write_del(self)?;
            write!(self.output(), "» -/> «")?;
        }
        write_ins(self)?;
        write!(self.output(), "»]")
    }

    fn write_ord_conflict(
        &mut self,
        write_confl_iter: impl Iterator<Item = impl FnOnce(&mut Self) -> Result>,
    ) -> Result {
        write!(self.output(), "INSERT_ORDER_CONFLICT![")?;
        for (i, write_ins) in write_confl_iter.enumerate() {
            if i == 0 {
                write!(self.output(), "«")?
            } else {
                write!(self.output(), ", «")?
            }
            write_ins(self)?;
            write!(self.output(), "»")?;
        }
        write!(self.output(), "]")
    }
}

pub struct PlainTreeFormatter<O> {
    output: O,
}

impl<O> PlainTreeFormatter<O> {
    pub fn new(output: O) -> Self {
        PlainTreeFormatter { output }
    }
}

impl<'o, O: std::io::Write> TreeFormatter for PlainTreeFormatter<O> {
    type Output = O;
    fn output(&mut self) -> &mut O {
        &mut self.output
    }
}

pub struct ColoredTreeFormatter<O> {
    output: O,
    parent_colors: ColorSet,
}

impl<O> ColoredTreeFormatter<O> {
    pub fn new(output: O) -> Self {
        ColoredTreeFormatter {
            output,
            parent_colors: ColorSet::white(),
        }
    }
}

impl<'o, O: std::io::Write> TreeFormatter for ColoredTreeFormatter<O> {
    type Output = O;
    fn output(&mut self) -> &mut O {
        &mut self.output
    }

    fn write_colored(
        &mut self,
        colors: ColorSet,
        write_tree: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        if colors == self.parent_colors {
            write_tree(self)
        } else {
            write!(
                self.output(),
                "{}:“",
                if colors == ColorSet::white() {
                    "ø".to_owned()
                } else {
                    colors
                        .iter()
                        .map(|c| format!("{}", c))
                        .collect::<Vec<_>>()
                        .join("&")
                }
            )?;
            let prev_parent_colors = self.parent_colors;
            self.parent_colors = colors;
            write_tree(self)?;
            self.parent_colors = prev_parent_colors;
            write!(self.output(), "”")
        }
    }
}
