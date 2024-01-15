pub trait Parse: std::fmt::Debug {
    fn parse(&self, src: &[u8], dst: Vec<u8>) -> std::io::Result<()>;

    fn as_dyn(&self) -> &dyn Parse
    where
        Self: Sized,
    {
        self
    }
}

pub trait AndThenParser<'a, P> {
    fn and_then(&mut self, parser: &'a P) -> &Self;
}

#[derive(Debug)]
pub struct Markdown<'a> {
    inner: Option<&'a dyn Parse>,
}

impl<'a> Markdown<'a> {
    pub fn new() -> Self {
        Self { inner: None }
    }
}

impl<'a, P: Parse> AndThenParser<'a, P> for Markdown<'a> {
    fn and_then(&mut self, parser: &'a P) -> &Self {
        self.inner = Some(parser.as_dyn());
        self
    }
}

impl<'a> Parse for Markdown<'a> {
    fn parse(&self, src: &[u8], dst: Vec<u8>) -> std::io::Result<()> {
        self.inner.as_ref().map(|p| p.parse(src, dst));
        Ok(())
    }
}

impl<'a> Iterator for Markdown<'a> {
    type Item = &'a dyn Parse;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
    }
}

#[derive(Debug, Default)]
pub struct Html<'a> {
    inner: Option<&'a dyn Parse>,
    pub tera: tera::Tera,
    context: tera::Context,
}

impl<'a> Html<'a> {
    pub fn new(tera: tera::Tera, context: tera::Context) -> Self {
        Self {
            inner: None,
            tera,
            context,
        }
    }
}

impl<'a, P: Parse> AndThenParser<'a, P> for Html<'a> {
    fn and_then(&mut self, parser: &'a P) -> &Self {
        self.inner = Some(parser.as_dyn());
        self
    }
}

impl<'a> Parse for Html<'a> {
    fn parse(&self, src: &[u8], dst: Vec<u8>) -> std::io::Result<()> {
        self.inner.as_ref().map(|p| p.parse(src, dst));
        Ok(())
    }
}

impl<'a> Iterator for Html<'a> {
    type Item = &'a dyn Parse;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
    }
}

pub struct Roxy;

impl Roxy {}

#[cfg(test)]
mod tests {
    use crate::roxy::{Html, Markdown, AndThenParser};

    #[test]
    fn iterate_parsers() {
        let parser = Markdown::new().and_then(&Html::default());

        println!("{parser:?}");
    }
}
