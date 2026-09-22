//! Plan §3.2: text nodes are escaped. Checked by decoding, not by trusting
//! the encoder; and the renderer contract of `site` holds.

use adapter_html::{HtmlRenderer, escape};
use library::testing::MemorySources;
use proptest::prelude::*;
use site::assembly::{Assemble, AssembleContext, DefaultAssembler};

fn decode(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

proptest! {
    #[test]
    fn escaping_round_trips_and_leaves_no_markup(text in any::<String>()) {
        let e = escape(&text);
        prop_assert!(!e.contains('<') && !e.contains('>') && !e.contains('"'));
        prop_assert_eq!(decode(&e), text);
    }
}

#[test]
fn the_renderer_meets_the_site_contract_and_escapes_page_text() {
    let (corpus, _) = library::load(&MemorySources::new(&[
        (
            "a.md",
            "# <script>alert(1)</script>\n\n[[x/b#one]] [[missing]]\n\n```mermaid\ngraph TD\n```\n",
        ),
        ("x/b.md", "# B\n## One\n"),
    ]))
    .expect("loads");
    let (docs, graph) = site::analyse(&corpus);
    let none = |_: &library::PageId| None;
    let pages: Vec<_> = docs
        .iter()
        .map(|d| DefaultAssembler.assemble(d, &graph, &none, &AssembleContext::default()))
        .collect();
    assert!(site::contract::page_renderer(&HtmlRenderer, &pages).is_empty());
    let a = String::from_utf8(site::PageRenderer::render(&HtmlRenderer, &pages[0]).bytes)
        .expect("utf8");
    assert!(!a.contains("<script>"), "{a}");
    assert!(a.contains("href=\"x/b.html#one\""), "{a}");
    assert!(a.contains("<span class=\"broken\">"), "{a}");
    assert!(a.contains("<pre class=\"mermaid\">"), "{a}");
}
