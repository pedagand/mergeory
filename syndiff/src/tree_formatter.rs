use crate::{Color, ColorSet, Metavariable};
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

    fn write_tag(
        &mut self,
        tag_name: &str,
        write_content: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        write!(self.output(), "{}![", tag_name.to_uppercase())?;
        write_content(self)?;
        write!(self.output(), "]")
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
        self.write_tag("changed", |fmt| {
            write!(fmt.output(), "«")?;
            write_del(fmt)?;
            write!(fmt.output(), "» -> «")?;
            write_ins(fmt)?;
            write!(fmt.output(), "»")
        })
    }

    fn write_deleted(&mut self, write_del: impl FnOnce(&mut Self) -> Result) -> Result {
        self.write_tag("deleted", write_del)
    }

    fn write_inserted(&mut self, write_ins: impl FnOnce(&mut Self) -> Result) -> Result {
        self.write_tag("inserted", write_ins)
    }

    fn write_mv_conflict(
        &mut self,
        mv: Metavariable,
        write_del: impl FnOnce(&mut Self) -> Result,
        write_repl: Option<impl FnOnce(&mut Self) -> Result>,
    ) -> Result {
        self.write_tag("mv_conflict", |fmt| {
            write!(fmt.output(), "${}: «", mv.0)?;
            write_del(fmt)?;
            write!(fmt.output(), "»")?;
            if let Some(write_repl) = write_repl {
                write!(fmt.output(), " <- «")?;
                write_repl(fmt)?;
                write!(fmt.output(), "»")?;
            }
            Ok(())
        })
    }

    fn write_ins_conflict(
        &mut self,
        write_confl_iter: impl Iterator<Item = impl FnOnce(&mut Self) -> Result>,
    ) -> Result {
        self.write_tag("conflict", |fmt| {
            for (i, write_ins) in write_confl_iter.enumerate() {
                if i == 0 {
                    write!(fmt.output(), "«")?
                } else {
                    write!(fmt.output(), ", «")?
                }
                write_ins(fmt)?;
                write!(fmt.output(), "»")?;
            }
            Ok(())
        })
    }

    fn write_del_conflict(
        &mut self,
        write_del: Option<impl FnOnce(&mut Self) -> Result>,
        write_ins: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        self.write_tag("delete_conflict", |fmt| {
            write!(fmt.output(), "«")?;
            if let Some(write_del) = write_del {
                write_del(fmt)?;
                write!(fmt.output(), "» -/> «")?;
            }
            write_ins(fmt)?;
            write!(fmt.output(), "»")
        })
    }

    fn write_ord_conflict(
        &mut self,
        write_confl_iter: impl Iterator<Item = impl FnOnce(&mut Self) -> Result>,
    ) -> Result {
        self.write_tag("insert_order_conflict", |fmt| {
            for (i, write_ins) in write_confl_iter.enumerate() {
                if i == 0 {
                    write!(fmt.output(), "«")?
                } else {
                    write!(fmt.output(), ", «")?
                }
                write_ins(fmt)?;
                write!(fmt.output(), "»")?;
            }
            Ok(())
        })
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

pub struct TextColoredTreeFormatter<O> {
    output: O,
    parent_colors: ColorSet,
}

impl<O> TextColoredTreeFormatter<O> {
    pub fn new(output: O) -> Self {
        TextColoredTreeFormatter {
            output,
            parent_colors: ColorSet::white(),
        }
    }
}

impl<O: std::io::Write> TreeFormatter for TextColoredTreeFormatter<O> {
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
                "“{}: ",
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

pub struct AnsiColoredTreeFormatter<O> {
    output: O,
    parent_colors: ColorSet,
    cur_style: ansi_term::Style,
}

impl<O> AnsiColoredTreeFormatter<O> {
    pub fn new(output: O) -> Self {
        AnsiColoredTreeFormatter {
            output,
            parent_colors: ColorSet::white(),
            cur_style: ansi_term::Style::new(),
        }
    }
}

const ANSI_COLORS: &[ansi_term::Color] = &[
    ansi_term::Color::Yellow,
    ansi_term::Color::Cyan,
    ansi_term::Color::Purple,
    ansi_term::Color::Blue,
    ansi_term::Color::Green,
    ansi_term::Color::Red,
];

fn style_for_color(color: Color) -> ansi_term::Style {
    ANSI_COLORS[usize::from(color)].normal()
}

impl<O: std::io::Write> TreeFormatter for AnsiColoredTreeFormatter<O> {
    type Output = O;
    fn output(&mut self) -> &mut O {
        &mut self.output
    }

    fn write_metavariable(&mut self, mv: Metavariable) -> Result {
        self.write_with_style(self.cur_style.underline(), |fmt| {
            write!(fmt.output(), "${}", mv.0)
        })
    }

    fn write_tag(
        &mut self,
        tag_name: &str,
        write_content: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        self.write_with_style(self.cur_style.bold(), |fmt| {
            write!(fmt.output(), "{}![", tag_name.to_uppercase())
        })?;
        write_content(self)?;
        self.write_with_style(self.cur_style.bold(), |fmt| write!(fmt.output(), "]"))
    }

    fn write_colored(
        &mut self,
        colors: ColorSet,
        write_tree: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        if colors == self.parent_colors {
            write_tree(self)
        } else {
            let prev_parent_colors = self.parent_colors;
            self.parent_colors = colors;
            if colors == ColorSet::white() {
                self.write_with_style(ansi_term::Style::new(), write_tree)?;
            } else if colors.iter().count() == 1 {
                let color = colors.iter().next().unwrap();
                self.write_with_style(style_for_color(color), write_tree)?;
            } else {
                self.write_color_quoted(colors.iter(), write_tree)?;
            }
            self.parent_colors = prev_parent_colors;
            Ok(())
        }
    }
}

impl<O: std::io::Write> AnsiColoredTreeFormatter<O> {
    fn write_with_style(
        &mut self,
        style: ansi_term::Style,
        write_fn: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        let prev_style = self.cur_style;
        self.cur_style = style;
        write!(self.output(), "{}", prev_style.infix(style))?;
        write_fn(self)?;
        self.cur_style = prev_style;
        write!(self.output(), "{}", style.infix(prev_style))
    }

    fn write_color_quoted(
        &mut self,
        mut color_iter: impl Iterator<Item = Color>,
        write_tree: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        match color_iter.next() {
            Some(color) => self.write_with_style(style_for_color(color), move |fmt| {
                write!(fmt.output(), "“")?;
                fmt.write_color_quoted(color_iter, write_tree)?;
                write!(fmt.output(), "”")
            }),
            None => self.write_with_style(ansi_term::Style::new(), write_tree),
        }
    }
}
