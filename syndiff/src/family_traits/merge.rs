use syn::punctuated::Punctuated;

pub trait Merge<I1, I2, O> {
    fn can_merge(&mut self, in1: &I1, in2: &I2) -> bool;
    fn merge(&mut self, in1: I1, in2: I2) -> O;
}

impl<I1, I2, O, M: Merge<I1, I2, O>> Merge<Box<I1>, Box<I2>, Box<O>> for M {
    fn can_merge(&mut self, in1: &Box<I1>, in2: &Box<I2>) -> bool {
        self.can_merge(&*in1, &*in2)
    }

    fn merge(&mut self, in1: Box<I1>, in2: Box<I2>) -> Box<O> {
        Box::new(self.merge(*in1, *in2))
    }
}

impl<I1, I2, O, M: Merge<I1, I2, O>> Merge<Vec<I1>, Vec<I2>, Vec<O>> for M {
    fn can_merge(&mut self, in1: &Vec<I1>, in2: &Vec<I2>) -> bool {
        if in1.len() != in2.len() {
            false
        } else {
            in1.iter()
                .zip(in2)
                .fold(true, |acc, (e1, e2)| acc && self.can_merge(&e1, &e2))
        }
    }

    fn merge(&mut self, in1: Vec<I1>, in2: Vec<I2>) -> Vec<O> {
        in1.into_iter()
            .zip(in2)
            .map(|(e1, e2)| self.merge(e1, e2))
            .collect()
    }
}

impl<I1, I2, O, M: Merge<I1, I2, O>> Merge<Option<I1>, Option<I2>, Option<O>> for M {
    fn can_merge(&mut self, in1: &Option<I1>, in2: &Option<I2>) -> bool {
        match (in1, in2) {
            (Some(e1), Some(e2)) => self.can_merge(e1, e2),
            (None, None) => true,
            _ => false,
        }
    }

    fn merge(&mut self, in1: Option<I1>, in2: Option<I2>) -> Option<O> {
        match (in1, in2) {
            (Some(e1), Some(e2)) => Some(self.merge(e1, e2)),
            (None, None) => None,
            _ => panic!("Trying to merge Some with None"),
        }
    }
}

impl<I1, I2, O, P: Default, M: Merge<I1, I2, O>>
    Merge<Punctuated<I1, P>, Punctuated<I2, P>, Punctuated<O, P>> for M
{
    fn can_merge(&mut self, in1: &Punctuated<I1, P>, in2: &Punctuated<I2, P>) -> bool {
        if in1.len() != in2.len() {
            false
        } else {
            in1.iter()
                .zip(in2)
                .fold(true, |acc, (e1, e2)| acc && self.can_merge(&e1, &e2))
        }
    }

    fn merge(&mut self, in1: Punctuated<I1, P>, in2: Punctuated<I2, P>) -> Punctuated<O, P> {
        in1.into_iter()
            .zip(in2)
            .map(|(e1, e2)| self.merge(e1, e2))
            .collect()
    }
}

impl<I1, I2, O, Tok, M: Merge<I1, I2, O>> Merge<(Tok, I1), (Tok, I2), (Tok, O)> for M {
    fn can_merge(&mut self, in1: &(Tok, I1), in2: &(Tok, I2)) -> bool {
        self.can_merge(&in1.1, &in2.1)
    }

    fn merge(&mut self, in1: (Tok, I1), in2: (Tok, I2)) -> (Tok, O) {
        (in1.0, self.merge(in1.1, in2.1))
    }
}

impl<I1, I2, O, Tok1, Tok2, M: Merge<I1, I2, O>>
    Merge<(Tok1, I1, Tok2), (Tok1, I2, Tok2), (Tok1, O, Tok2)> for M
{
    fn can_merge(&mut self, in1: &(Tok1, I1, Tok2), in2: &(Tok1, I2, Tok2)) -> bool {
        self.can_merge(&in1.1, &in2.1)
    }

    fn merge(&mut self, in1: (Tok1, I1, Tok2), in2: (Tok1, I2, Tok2)) -> (Tok1, O, Tok2) {
        (in1.0, self.merge(in1.1, in2.1), in1.2)
    }
}
