use galvan_ast::TypeIdent;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct RustFunctionId(Box<str>);

impl RustFunctionId {
    pub(super) fn new(receiver: Option<&TypeIdent>, name: &str, labels: &[&str]) -> Self {
        let mut id = String::new();
        if let Some(receiver) = receiver {
            id.push_str(receiver.as_str());
            id.push_str("::");
        }
        id.push_str(name);
        if !labels.is_empty() {
            id.push(':');
            id.push_str(&labels.join(":"));
        }
        Self(id.into())
    }
}
