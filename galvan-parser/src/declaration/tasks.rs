#[derive(Debug)]
pub struct MainDecl {
    // TODO: body
}

#[derive(Debug)]
pub struct TestDecl {
    name: Option<String>,
}

#[derive(Debug)]
pub struct TaskDecl {
    keyword: TaskKeyword,
    name: Option<String>,
    // TODO: body
}

#[derive(Debug)]
pub enum TaskKeyword {
    Main,
    Test,
    Build,
    Custom(String),
}
