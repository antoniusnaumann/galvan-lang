use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Source {
    File { path: String, content: Arc<str> },
    Str(Arc<str>),
}

impl Source {
    pub fn from_string(string: impl Into<Arc<str>>) -> Source {
        Self::Str(string.into())
    }

    pub fn read(path: impl AsRef<Path>) -> Source {
        Self::File {
            path: path.as_ref().to_string_lossy().into(),
            content: fs::read_to_string(path).unwrap().into(),
        }
    }

    pub fn content(&self) -> &str {
        match self {
            Self::File { path: _, content } => content.as_ref(),
            Self::Str(content) => content.as_ref(),
        }
    }

    pub fn origin(&self) -> Option<&str> {
        match self {
            Self::File { path, content: _ } => Some(path),
            Self::Str(_) => None,
        }
    }
}

impl<T> From<T> for Source
where
    T: Into<Arc<str>>,
{
    fn from(value: T) -> Self {
        Self::from_string(value)
    }
}

impl Deref for Source {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.content()
    }
}
