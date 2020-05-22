use syn::punctuated::Punctuated;

pub trait VisitMut<T> {
    fn visit_mut(&mut self, input: &mut T);
}

impl<T, V: VisitMut<T>> VisitMut<Box<T>> for V {
    fn visit_mut(&mut self, input: &mut Box<T>) {
        self.visit_mut(&mut *input)
    }
}

impl<T, V: VisitMut<T>> VisitMut<Vec<T>> for V {
    fn visit_mut(&mut self, input: &mut Vec<T>) {
        for elt in input {
            self.visit_mut(elt)
        }
    }
}

impl<T, V: VisitMut<T>> VisitMut<Option<T>> for V {
    fn visit_mut(&mut self, input: &mut Option<T>) {
        match input {
            Some(v) => self.visit_mut(v),
            None => (),
        }
    }
}

impl<T, P, V: VisitMut<T>> VisitMut<Punctuated<T, P>> for V {
    fn visit_mut(&mut self, input: &mut Punctuated<T, P>) {
        for elt in input {
            self.visit_mut(elt)
        }
    }
}

impl<T, Tok, V: VisitMut<T>> VisitMut<(Tok, T)> for V {
    fn visit_mut(&mut self, input: &mut (Tok, T)) {
        self.visit_mut(&mut input.1)
    }
}

impl<T, Tok1, Tok2, V: VisitMut<T>> VisitMut<(Tok1, T, Tok2)> for V {
    fn visit_mut(&mut self, input: &mut (Tok1, T, Tok2)) {
        self.visit_mut(&mut input.1)
    }
}
