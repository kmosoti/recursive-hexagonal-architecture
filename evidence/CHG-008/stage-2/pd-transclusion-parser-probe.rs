use library::{RelPath,Source};
use pulldown_cmark::{Options,Parser};
fn main(){
 let samples=["![[Page]]",r"\![[Page]]","before ![[Page]] after","! [[Page]]","![[Page|label]]","`![[Page]]`","  ![[Page#section]]  ","- ![[Page]]","> ![[Page]]","![[Page]]\n![[Other]]","# Host\n> ## Nested\n> body\n\n## Next"];
 for text in samples{
  println!("INPUT {text:?}");
  for(event,range) in Parser::new_ext(text,Options::ENABLE_WIKILINKS|Options::ENABLE_GFM).into_offset_iter(){println!("  {event:?} {range:?} {:?}",&text[range.clone()]);}
  let doc=document::parse(&Source::new(RelPath::new("host.md").unwrap(),text.into()));
  println!("links {:?}; headings {:?}; body {:?}",doc.links,doc.headings,doc.body);
 }
}
