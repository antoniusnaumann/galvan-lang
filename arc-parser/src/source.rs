use std::{path::Path, sync::Arc};

pub enum Source {
    File { path: String, content: Arc<str> },
    Str(Arc<str>),
}

impl Source {
    pub fn from_string(string: impl Into<Arc<str>>) -> Source {
        Self::Str(string.into())
    }

    pub fn read(path: impl AsRef<Path>) -> Source {
        todo!("Read from file")
    }

    pub fn content(&self) -> &str {
        match self {
            Self::File { path: _, content } => content.as_ref(),
            Self::Str(content) => content.as_ref(),
        }
    }

    pub fn origin(&self) -> Option<&str> {
        match self {
            Self::File { path, content: _ } => Some(&path),
            Self::Str(_) => None,
        }
    }
}
