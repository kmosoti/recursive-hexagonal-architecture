//! In-process fakes for the site ports (plan §3.1), always compiled.

use std::collections::BTreeMap;

use library::{Digest, RelPath};

use crate::assembly::PageModel;
use crate::{Clock, OutputSink, PageRenderer, Rendered, SinkError};

/// A sink over a map.
#[derive(Debug, Clone, Default)]
pub struct RecordingSink {
    pub files: BTreeMap<RelPath, Vec<u8>>,
}

impl OutputSink for RecordingSink {
    fn list(&self) -> Result<Vec<(RelPath, Digest)>, SinkError> {
        Ok(self
            .files
            .iter()
            .map(|(p, b)| (p.clone(), Digest::of(b)))
            .collect())
    }
    fn write(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), SinkError> {
        self.files.insert(path.clone(), bytes.to_vec());
        Ok(())
    }
    fn delete(&mut self, path: &RelPath) -> Result<(), SinkError> {
        self.files.remove(path);
        Ok(())
    }
}

/// A clock that always shows the same time.
#[derive(Debug, Clone)]
pub struct FixedClock(pub String);

impl Clock for FixedClock {
    fn now(&self) -> String {
        self.0.clone()
    }
}

/// A renderer writing `<id>.txt` with the title and link count.
#[derive(Debug, Clone, Copy, Default)]
pub struct StubRenderer;

impl PageRenderer for StubRenderer {
    fn render(&self, page: &PageModel) -> Rendered {
        let name = format!("{}.txt", page.id);
        #[allow(clippy::expect_used)]
        let path = RelPath::new(&name).expect("page ids are relative paths");
        Rendered {
            path,
            bytes: format!("{}|{}", page.title, page.links.len()).into_bytes(),
        }
    }
    fn assets(&self) -> Vec<Rendered> {
        Vec::new()
    }
}
