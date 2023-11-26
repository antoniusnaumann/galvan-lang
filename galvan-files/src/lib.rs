mod source;
pub use source::*;

#[cfg(feature = "exec")]
mod exec;

#[cfg(feature = "exec")]
pub use exec::*;

trait GalvanFileExtension {
    fn has_galvan_extension(&self) -> bool;
}

impl GalvanFileExtension for std::path::Path {
    fn has_galvan_extension(&self) -> bool {
        self.extension() == Some("galvan".as_ref()) || self.extension() == Some("gv".as_ref())
    }
}
