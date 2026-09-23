/// The document tree. Owned, and independent of the parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Heading {
        level: u8,
        slug: String,
        children: Vec<Node>,
    },
    Paragraph(Vec<Node>),
    Text(String),
    Code(String),
    CodeBlock {
        lang: Option<String>,
        text: String,
    },
    Emphasis(Vec<Node>),
    Strong(Vec<Node>),
    Strikethrough(Vec<Node>),
    /// An ordinary markdown link.
    Link {
        href: String,
        children: Vec<Node>,
    },
    /// A wikilink; `index` is its position in [`crate::Document::links`].
    WikiLink {
        index: usize,
        children: Vec<Node>,
    },
    Transclusion(Transclusion),
    Image {
        src: String,
        alt: String,
    },
    List {
        start: Option<u64>,
        items: Vec<Vec<Node>>,
    },
    BlockQuote {
        kind: Option<CalloutKind>,
        children: Vec<Node>,
    },
    Table {
        align: Vec<Align>,
        head: Vec<Vec<Node>>,
        rows: Vec<Vec<Vec<Node>>>,
    },
    Rule,
    SoftBreak,
    HardBreak,
    /// Raw HTML, kept as text and never emitted as markup.
    Html(String),
    TaskMarker(bool),
}

/// A GFM callout's kind (preserved; rendering is plain in Phase 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalloutKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

/// Table column alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

/// A parser-recognized transclusion embed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transclusion {
    pub id: usize,
    pub target: String,
    pub anchor: Option<String>,
    pub display: String,
    pub line: usize,
}
