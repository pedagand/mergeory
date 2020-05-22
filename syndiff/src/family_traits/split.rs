use syn::punctuated::Punctuated;

pub trait Split<In, Out1, Out2> {
    fn split(&mut self, input: In) -> (Out1, Out2);
}

impl<In, Out1, Out2, T: Split<In, Out1, Out2>> Split<Box<In>, Box<Out1>, Box<Out2>> for T {
    fn split(&mut self, input: Box<In>) -> (Box<Out1>, Box<Out2>) {
        let (out1, out2) = self.split(*input);
        (Box::new(out1), Box::new(out2))
    }
}

impl<In, Out1, Out2, T: Split<In, Out1, Out2>> Split<Vec<In>, Vec<Out1>, Vec<Out2>> for T {
    fn split(&mut self, input: Vec<In>) -> (Vec<Out1>, Vec<Out2>) {
        input.into_iter().map(|elt| self.split(elt)).unzip()
    }
}

impl<In, Out1, Out2, T: Split<In, Out1, Out2>> Split<Option<In>, Option<Out1>, Option<Out2>> for T {
    fn split(&mut self, input: Option<In>) -> (Option<Out1>, Option<Out2>) {
        match input {
            Some(i) => {
                let (o1, o2) = self.split(i);
                (Some(o1), Some(o2))
            }
            None => (None, None),
        }
    }
}

impl<In, Out1, Out2, P: Default, T: Split<In, Out1, Out2>>
    Split<Punctuated<In, P>, Punctuated<Out1, P>, Punctuated<Out2, P>> for T
{
    fn split(&mut self, input: Punctuated<In, P>) -> (Punctuated<Out1, P>, Punctuated<Out2, P>) {
        input.into_iter().map(|v| self.split(v)).unzip()
    }
}

impl<In, Out1, Out2, Tok: Copy, T: Split<In, Out1, Out2>> Split<(Tok, In), (Tok, Out1), (Tok, Out2)>
    for T
{
    fn split(&mut self, (tok, input): (Tok, In)) -> ((Tok, Out1), (Tok, Out2)) {
        let (o1, o2) = self.split(input);
        ((tok, o1), (tok, o2))
    }
}

impl<In, Out1, Out2, Tok1: Copy, Tok2: Copy, T: Split<In, Out1, Out2>>
    Split<(Tok1, In, Tok2), (Tok1, Out1, Tok2), (Tok1, Out2, Tok2)> for T
{
    fn split(
        &mut self,
        (tok1, input, tok2): (Tok1, In, Tok2),
    ) -> ((Tok1, Out1, Tok2), (Tok1, Out2, Tok2)) {
        let (o1, o2) = self.split(input);
        ((tok1, o1, tok2), (tok1, o2, tok2))
    }
}
