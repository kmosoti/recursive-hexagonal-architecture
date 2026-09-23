use library::{Corpus,RelPath,Source};
fn main() {
 let sources = vec![Source::new(RelPath::new("same.md").unwrap(),"# Alpha".into()), Source::new(RelPath::new("same.md").unwrap(),"# Beta".into()), Source::new(RelPath::new("index.md").unwrap(),"[[same#alpha]]".into())];
 assert!(Corpus::new(sources.clone()).is_err());
 let mut docs:Vec<_>=sources.iter().map(document::parse).collect();
 let before=graph::resolve(&docs); docs.reverse(); let after=graph::resolve(&docs);
 println!("Duplicate ids rejected by Corpus; graph slice with duplicates is outside that domain. Before witnesses: {:?}; reversed: {:?}",before.witnesses,after.witnesses);
 assert_ne!(before,after);
}
