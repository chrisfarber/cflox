#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[allow(dead_code)]
    pub fn in_source<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub span: Span,
    pub node: T,
}

impl<T> PartialEq for Spanned<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T> Spanned<T> {
    pub fn encapsulating(l: impl Into<Span>, r: impl Into<Span>, node: T) -> Spanned<T> {
        let start = l.into().start;
        let end = r.into().end;
        Self {
            span: Span { start, end },
            node,
        }
    }
}

impl<T> From<&Spanned<T>> for Span {
    fn from(spanned: &Spanned<T>) -> Span {
        spanned.span
    }
}
