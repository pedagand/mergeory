#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Color(u8);

impl TryFrom<usize> for Color {
    type Error = ();
    fn try_from(value: usize) -> Result<Color, ()> {
        if value < 64 {
            Ok(Color(value as u8))
        } else {
            Err(())
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ColorSet(u64);

impl ColorSet {
    pub fn white() -> ColorSet {
        ColorSet(0)
    }

    pub fn single_color(color: Color) -> ColorSet {
        ColorSet(1 << color.0)
    }

    pub fn contains(&self, color: Color) -> bool {
        self.0 & (1 << color.0) != 0
    }

    pub fn color_list(&self) -> Vec<Color> {
        let mut list = Vec::new();
        for c in 0..64 {
            if self.contains(Color(c)) {
                list.push(Color(c))
            }
        }
        list
    }
}

impl std::ops::BitOr for ColorSet {
    type Output = ColorSet;
    fn bitor(self, rhs: ColorSet) -> ColorSet {
        ColorSet(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for ColorSet {
    fn bitor_assign(&mut self, rhs: ColorSet) {
        self.0 |= rhs.0
    }
}

#[derive(Clone, Copy)]
pub struct Colored<T> {
    pub data: T,
    pub colors: ColorSet,
}

impl<T> Colored<T> {
    pub fn new_white(data: T) -> Colored<T> {
        Colored {
            data,
            colors: ColorSet::white(),
        }
    }

    pub fn with_color(color: Color, data: T) -> Colored<T> {
        Colored {
            data,
            colors: ColorSet::single_color(color),
        }
    }

    pub fn as_ref(&self) -> Colored<&T> {
        Colored {
            data: &self.data,
            colors: self.colors,
        }
    }

    pub fn merge<L, R>(
        left: Colored<L>,
        right: Colored<R>,
        merge_fn: impl FnOnce(L, R) -> Option<T>,
    ) -> Option<Colored<T>> {
        Some(Colored {
            data: merge_fn(left.data, right.data)?,
            colors: left.colors | right.colors,
        })
    }
}
