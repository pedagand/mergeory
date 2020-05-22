use syn::punctuated::Punctuated;

pub trait Visit<T> {
    fn visit(&mut self, input: &T);
}

impl<T, V: Visit<T>> Visit<Box<T>> for V {
    fn visit(&mut self, input: &Box<T>) {
        self.visit(&*input)
    }
}

impl<T, V: Visit<T>> Visit<Vec<T>> for V {
    fn visit(&mut self, input: &Vec<T>) {
        for elt in input {
            self.visit(elt)
        }
    }
}

impl<T, V: Visit<T>> Visit<Option<T>> for V {
    fn visit(&mut self, input: &Option<T>) {
        match input {
            Some(v) => self.visit(v),
            None => (),
        }
    }
}

impl<T, P, V: Visit<T>> Visit<Punctuated<T, P>> for V {
    fn visit(&mut self, input: &Punctuated<T, P>) {
        for elt in input {
            self.visit(elt)
        }
    }
}

impl<T, Tok, V: Visit<T>> Visit<(Tok, T)> for V {
    fn visit(&mut self, input: &(Tok, T)) {
        self.visit(&input.1)
    }
}

impl<T, Tok1, Tok2, V: Visit<T>> Visit<(Tok1, T, Tok2)> for V {
    fn visit(&mut self, input: &(Tok1, T, Tok2)) {
        self.visit(&input.1)
    }
}
