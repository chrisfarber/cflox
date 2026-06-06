use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_NODE_ID: AtomicU64 = AtomicU64::new(0);

pub type NodeId = u64;

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

impl From<usize> for Span {
    fn from(size: usize) -> Span {
        Span {
            start: size,
            end: size,
        }
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

impl<T> From<&Spanned<T>> for Span {
    fn from(spanned: &Spanned<T>) -> Span {
        spanned.span
    }
}

#[derive(Debug, Clone)]
pub struct Node<T> {
    id: NodeId,
    pub span: Span,
    pub node: T,
}

impl<T> PartialEq for Node<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T> Node<T> {
    pub fn new(span: impl Into<Span>, node: T) -> Node<T> {
        Self {
            id: NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed),
            span: span.into(),
            node,
        }
    }

    pub fn encapsulating(l: impl Into<Span>, r: impl Into<Span>, node: T) -> Node<T> {
        let start = l.into().start;
        let end = r.into().end;
        Self::new(Span { start, end }, node)
    }

    #[allow(dead_code)]
    pub fn untracked(node: T) -> Self {
        Self::new(Span { start: 0, end: 0 }, node)
    }

    pub fn id(&self) -> NodeId {
        self.id
    }
}

impl<T> From<&Node<T>> for Span {
    fn from(spanned: &Node<T>) -> Span {
        spanned.span
    }
}
