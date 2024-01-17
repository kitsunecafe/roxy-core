use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::Path,
};

pub trait Parse {
    fn parse(&mut self, path: &str, src: &[u8], dst: &mut Vec<u8>) -> std::io::Result<()>;
}

pub trait AndThenParser<P> {
    fn and_then(&mut self, parser: P) -> &Self;
}

pub struct Parser {
    steps: Vec<Box<dyn Parse>>,
}

impl Parser {
    pub fn new() -> Self {
        Parser { steps: Vec::new() }
    }

    pub fn push<P: Parse + 'static>(&mut self, parser: P) {
        self.steps.push(Box::new(parser));
    }
}

impl Parse for Parser {
    fn parse(&mut self, path: &str, src: &[u8], dst: &mut Vec<u8>) -> std::io::Result<()> {
        let mut buf_1 = Vec::from(src);
        let mut buf_2 = Vec::from(dst.as_slice());

        self.steps
            .iter_mut()
            .try_fold((&mut buf_1, &mut buf_2), |(src, mut dst), p| {
                dst.clear();
                p.parse(path, src.as_slice(), &mut dst).map(|()| (dst, src))
            })
            .map(|(a, _)| dst.write_all(a))?
    }
}

#[derive(Debug)]
pub struct Markdown;

impl Markdown {
    pub fn new() -> Self {
        Self
    }
}

impl Parse for Markdown {
    fn parse(&mut self, _path: &str, src: &[u8], dst: &mut Vec<u8>) -> std::io::Result<()> {
        let src = String::from_utf8_lossy(src).to_string();
        let parser = pulldown_cmark::Parser::new(src.as_str());
        pulldown_cmark::html::write_html(dst, parser)
    }
}

#[derive(Debug, Default)]
pub struct Html {
    pub tera: tera::Tera,
    context: tera::Context,
}

impl Html {
    pub fn new(tera: tera::Tera, context: tera::Context) -> Self {
        Self { tera, context }
    }
}

impl Parse for Html {
    fn parse(&mut self, path: &str, src: &[u8], dst: &mut Vec<u8>) -> std::io::Result<()> {
        // TODO: This error is a hack
        let err = |_| std::io::Error::new(std::io::ErrorKind::InvalidData, "fail");
        let template = String::from_utf8_lossy(src).to_string();

        self.tera
            .add_raw_template(path, template.as_str())
            .map_err(err)?;

        self.tera.render_to(path, &self.context, dst).map_err(err)
    }
}

pub struct Asset<'a, R> {
    path: &'a str,
    data: R,
}

impl<'a, R> Asset<'a, R> {
    pub fn new(path: &'a str, data: R) -> Self {
        Asset { path, data }
    }
}

impl<'a> TryFrom<&'a str> for Asset<'a, BufReader<File>> {
    type Error = std::io::Error;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        File::open(value)
            .map(BufReader::new)
            .map(|data| Self::new(value, data))
    }
}

impl<'a, R: Read> Read for Asset<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.data.read(buf)
    }
}

impl<'a, R: BufRead> BufRead for Asset<'a, R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.data.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.data.consume(amt)
    }
}

pub struct Roxy;

impl Roxy {
    pub fn load(path: &str) -> std::io::Result<Asset<BufReader<File>>> {
        path.try_into()
    }

    fn read_asset(
        asset: &mut Asset<BufReader<File>>,
        parser: &mut Parser,
    ) -> std::io::Result<Vec<u8>> {
        let mut src = Vec::new();
        let mut dst = Vec::new();

        asset.data.read_to_end(&mut src)?;

        parser.parse(asset.path, &src, &mut dst)?;

        Ok(dst)
    }

    fn path_to_str<P: AsRef<Path>>(path: &P) -> std::io::Result<&str> {
        path.as_ref()
            .to_str()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "{path} is invalid"))
    }

    pub fn parse<P: AsRef<Path>>(path: &P, parser: &mut Parser) -> std::io::Result<Vec<u8>> {
        Self::path_to_str(path)
            .and_then(Self::load)
            .and_then(|mut a| Self::read_asset(&mut a, parser))
    }

    fn mkdir_and_open<P: AsRef<Path>>(path: &P) -> std::io::Result<File> {
        let path = path.as_ref();
        fs::create_dir_all(path.with_file_name(""))?;
        File::create(path)
    }

    pub fn process_file<P: AsRef<Path>>(
        input: &P,
        output: &P,
        parser: &mut Parser,
    ) -> std::io::Result<()> {
        let buf = Self::parse(input, parser)?;
        Self::mkdir_and_open(&output).and_then(|mut f| f.write_all(&buf))
    }
}

#[cfg(test)]
mod tests {
    use crate::roxy::{Html, Markdown, Parser};

    use super::Parse;

    #[test]
    fn md_and_html() {
        let mut parsers = Parser::new();
        parsers.push(Markdown::new());
        let mut ctx = tera::Context::new();

        ctx.insert("test", "fox");
        parsers.push(Html::new(tera::Tera::default(), ctx));

        let mut buf = Vec::new();

        parsers
            .parse("test.html", b"# {{ test }} :3", &mut buf)
            .unwrap();

        assert_eq!(String::from_utf8_lossy(buf.as_slice()), "<h1>fox :3</h1>\n");
    }
}
