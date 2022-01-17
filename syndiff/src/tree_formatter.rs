use crate::merge::Color;
use crate::Metavariable;
use std::io::Write;

type Result = std::io::Result<()>;

pub enum ChangeType {
    Deletion,
    Insertion,
    Inline,
}

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
        _color: Color,
        write_tree: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        write_tree(self)
    }

    fn write_change_tree(
        &mut self,
        _typ: ChangeType,
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
            fmt.write_change_tree(ChangeType::Deletion, write_del)?;
            write!(fmt.output(), "» -> «")?;
            fmt.write_change_tree(ChangeType::Insertion, write_ins)?;
            write!(fmt.output(), "»")
        })
    }

    fn write_deleted(&mut self, write_del: impl FnOnce(&mut Self) -> Result) -> Result {
        self.write_tag("deleted", |fmt| {
            fmt.write_change_tree(ChangeType::Deletion, write_del)
        })
    }

    fn write_inserted(&mut self, write_ins: impl FnOnce(&mut Self) -> Result) -> Result {
        self.write_tag("inserted", |fmt| {
            fmt.write_change_tree(ChangeType::Insertion, write_ins)
        })
    }

    fn write_mv_conflict(
        &mut self,
        mv: Metavariable,
        write_del: impl FnOnce(&mut Self) -> Result,
        write_repl: Option<impl FnOnce(&mut Self) -> Result>,
    ) -> Result {
        self.write_tag("mv_conflict", |fmt| {
            fmt.write_metavariable(mv)?;
            write!(fmt.output(), ": «")?;
            fmt.write_change_tree(ChangeType::Deletion, write_del)?;
            write!(fmt.output(), "»")?;
            if let Some(write_repl) = write_repl {
                write!(fmt.output(), " <- «")?;
                fmt.write_change_tree(ChangeType::Inline, write_repl)?;
                write!(fmt.output(), "»")?;
            }
            Ok(())
        })
    }

    fn write_ins_conflict(
        &mut self,
        write_confl_left: impl FnOnce(&mut Self) -> Result,
        write_confl_right: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        self.write_tag("conflict", |fmt| {
            write!(fmt.output(), "«")?;
            fmt.write_change_tree(ChangeType::Insertion, write_confl_left)?;
            write!(fmt.output(), "», «")?;
            fmt.write_change_tree(ChangeType::Insertion, write_confl_right)?;
            write!(fmt.output(), "»")
        })
    }

    fn write_del_conflict(
        &mut self,
        write_del: impl FnOnce(&mut Self) -> Result,
        write_ins: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        self.write_tag("delete_conflict", |fmt| {
            write!(fmt.output(), "«")?;
            fmt.write_change_tree(ChangeType::Deletion, write_del)?;
            write!(fmt.output(), "» -/> «")?;
            fmt.write_change_tree(ChangeType::Insertion, write_ins)?;
            write!(fmt.output(), "»")
        })
    }

    fn write_ord_conflict(
        &mut self,
        write_confl_left: impl FnOnce(&mut Self) -> Result,
        write_confl_right: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        self.write_tag("insert_order_conflict", |fmt| {
            write!(fmt.output(), "«")?;
            fmt.write_change_tree(ChangeType::Insertion, write_confl_left)?;
            write!(fmt.output(), "», «")?;
            fmt.write_change_tree(ChangeType::Insertion, write_confl_right)?;
            write!(fmt.output(), "»")
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
    parent_color: Color,
}

impl<O> TextColoredTreeFormatter<O> {
    pub fn new(output: O) -> Self {
        TextColoredTreeFormatter {
            output,
            parent_color: Color::White,
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
        color: Color,
        write_tree: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        if color == self.parent_color {
            write_tree(self)
        } else {
            let color_sym = match color {
                Color::White => '#',
                Color::Left => '<',
                Color::Right => '>',
                Color::Both => '=',
            };
            write!(self.output(), "“{}", color_sym)?;
            let prev_parent_color = self.parent_color;
            self.parent_color = color;
            write_tree(self)?;
            self.parent_color = prev_parent_color;
            write!(self.output(), "{}”", color_sym)
        }
    }
}

pub struct AnsiColoredTreeFormatter<O> {
    output: O,
    parent_color: Color,
    cur_style: ansi_term::Style,
    change_type: Option<ChangeType>,
}

impl<O> AnsiColoredTreeFormatter<O> {
    pub fn new(output: O) -> Self {
        AnsiColoredTreeFormatter {
            output,
            parent_color: Color::White,
            cur_style: ansi_term::Style::new(),
            change_type: None,
        }
    }
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
        self.write_with_style(ansi_term::Color::White.bold(), |fmt| {
            write!(fmt.output(), "{}![", tag_name.to_uppercase())?;
            write_content(fmt)?;
            write!(fmt.output(), "]")
        })
    }

    fn write_colored(
        &mut self,
        color: Color,
        write_tree: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        if color == self.parent_color {
            write_tree(self)
        } else {
            let prev_parent_color = self.parent_color;
            self.parent_color = color;
            self.write_with_style(self.style_for_color(color), write_tree)?;
            self.parent_color = prev_parent_color;
            Ok(())
        }
    }

    fn write_change_tree(
        &mut self,
        typ: ChangeType,
        write_tree: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        let prev_change_type = std::mem::replace(&mut self.change_type, Some(typ));
        let prev_parent_color = self.parent_color;
        self.parent_color = Color::Both; // Both by default to color single differences
        self.write_with_style(self.style_for_color(Color::Both), write_tree)?;
        self.parent_color = prev_parent_color;
        self.change_type = prev_change_type;
        Ok(())
    }
}

impl<O: std::io::Write> AnsiColoredTreeFormatter<O> {
    fn style_for_color(&self, color: Color) -> ansi_term::Style {
        match color {
            Color::White => ansi_term::Style::new(),
            Color::Left => ansi_term::Color::Yellow.bold(),
            Color::Right => ansi_term::Color::Cyan.bold(),
            Color::Both => match self.change_type.as_ref().unwrap() {
                ChangeType::Deletion => ansi_term::Color::Red.bold(),
                ChangeType::Insertion => ansi_term::Color::Green.bold(),
                ChangeType::Inline => ansi_term::Color::White.bold(),
            },
        }
    }

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
}

pub trait TreeFormattable {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> Result;
}

impl<T: TreeFormattable> TreeFormattable for Vec<T> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> Result {
        for item in self {
            item.write_with(fmt)?
        }
        Ok(())
    }
}
