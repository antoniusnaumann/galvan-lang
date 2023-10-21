pub struct MainDecl {
    // TODO: body
}

pub struct TestDecl {
    name: Option<String>,
}

pub struct TaskDecl {
    keyword: TaskKeyword,
    name: Option<String>,
    // TODO: body
}

pub enum TaskKeyword {
    Main,
    Test,
    Build,
    Custom(String),
}
