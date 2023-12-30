use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::GalvanFileExtension;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileError {
    #[error("Error when trying to read source file {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    #[error("File name {0} is not valid UTF-8")]
    Utf8(String),
    #[error("File name {0} is not allowed. Only lowercase letters and _ are allowed in galvan file names")]
    Naming(String),
    #[error("File {0} has no extension")]
    MissingExtension(PathBuf),
}

impl FileError {
    pub fn io(path: impl AsRef<Path>, error: std::io::Error) -> Self {
        Self::Io(path.as_ref().to_owned(), error)
    }

    pub fn utf8(file_name: impl Into<String>) -> Self {
        Self::Utf8(file_name.into())
    }

    pub fn naming(file_name: impl Into<String>) -> Self {
        Self::Naming(file_name.into())
    }

    pub fn missing_extension(path: impl AsRef<Path>) -> Self {
        Self::MissingExtension(path.as_ref().to_owned())
    }
}

pub type SourceResult = Result<Source, FileError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    File {
        path: Arc<Path>,
        content: Arc<str>,
        canonical_name: Arc<str>,
    },
    Str(Arc<str>),
    Missing,
    Builtin,
}

impl Source {
    pub fn from_string(string: impl Into<Arc<str>>) -> Source {
        Self::Str(string.into())
    }

    pub fn read(path: impl AsRef<Path>) -> SourceResult {
        let path = path.as_ref();
        if !path.has_galvan_extension() {
            Err(FileError::missing_extension(path))?
        }
        let stem = path
            .file_stem()
            .ok_or_else(|| FileError::missing_extension(path))?;

        let stem = stem
            .to_str()
            .ok_or_else(|| FileError::utf8(stem.to_string_lossy()))?;
        if !stem.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            Err(FileError::naming(stem))?
        }
        let canonical_name = stem.replace(".", "_").into();
        let content = fs::read_to_string(path)
            .map_err(|e| FileError::io(path, e))?
            .into();
        let path = path.into();

        Ok(Self::File {
            path,
            content,
            canonical_name,
        })
    }

    pub fn content(&self) -> &str {
        match self {
            Self::File { content, .. } => content.as_ref(),
            Self::Str(content) => content.as_ref(),
            Self::Missing => "",
            Self::Builtin => "",
        }
    }

    pub fn origin(&self) -> Option<&Path> {
        match self {
            Self::File { path, .. } => Some(path),
            Self::Str(_) => None,
            Self::Missing => None,
            Self::Builtin => None,
        }
    }

    pub fn canonical_name(&self) -> Option<&str> {
        match self {
            Self::File {
                path: _,
                content: _,
                canonical_name,
            } => Some(canonical_name),
            Self::Str(_) => None,
            Self::Missing => None,
            Self::Builtin => Some("galvan_std"),
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
