use syn::punctuated::Punctuated;

pub trait Convert<In, Out> {
    fn convert(&mut self, input: In) -> Out;
}

impl<In, Out, T: Convert<In, Out>> Convert<Box<In>, Box<Out>> for T {
    fn convert(&mut self, input: Box<In>) -> Box<Out> {
        Box::new(self.convert(*input))
    }
}

impl<In, Out, T: Convert<In, Out>> Convert<Vec<In>, Vec<Out>> for T {
    fn convert(&mut self, input: Vec<In>) -> Vec<Out> {
        input.into_iter().map(|v| self.convert(v)).collect()
    }
}

impl<In, Out, T: Convert<In, Out>> Convert<Option<In>, Option<Out>> for T {
    fn convert(&mut self, input: Option<In>) -> Option<Out> {
        input.map(|v| self.convert(v))
    }
}

impl<In, Out, P: Default, T: Convert<In, Out>> Convert<Punctuated<In, P>, Punctuated<Out, P>>
    for T
{
    fn convert(&mut self, input: Punctuated<In, P>) -> Punctuated<Out, P> {
        input.into_iter().map(|v| self.convert(v)).collect()
    }
}

impl<In, Out, Tok, T: Convert<In, Out>> Convert<(Tok, In), (Tok, Out)> for T {
    fn convert(&mut self, (tok, input): (Tok, In)) -> (Tok, Out) {
        (tok, self.convert(input))
    }
}

impl<In, Out, Tok1, Tok2, T: Convert<In, Out>> Convert<(Tok1, In, Tok2), (Tok1, Out, Tok2)> for T {
    fn convert(&mut self, (tok1, input, tok2): (Tok1, In, Tok2)) -> (Tok1, Out, Tok2) {
        (tok1, self.convert(input), tok2)
    }
}

impl<In, T> Convert<In, ()> for T {
    fn convert(&mut self, _: In) {}
}

// Rust will not let me write that because there is an overlapping implementation
// for Convert<(), ()>. This needs the WIP specialization system to be expressed.
/*impl<Out: Default, T> Convert<(), Out> for T {
    fn convert(&mut self, _: ()) -> Out {
        Out::default()
    }
}*/

impl<In: ToString, T> Convert<In, String> for T {
    fn convert(&mut self, input: In) -> String {
        input.to_string()
    }
}
