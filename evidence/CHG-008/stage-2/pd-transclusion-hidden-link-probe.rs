use library::{RelPath,Source};
fn main(){let text="![see [[Page]]](image.png)\n[[Next]]";let d=document::parse(&Source::new(RelPath::new("host.md").unwrap(),text.into()));println!("links: {:?}\nbody: {:?}",d.links,d.body);}
