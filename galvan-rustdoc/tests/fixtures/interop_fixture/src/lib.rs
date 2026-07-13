pub struct Payload {
    pub id: u64,
    pub name: String,
}

pub enum FixtureError {
    Invalid,
}

pub enum FixtureEvent<T> {
    Ready(T),
    Failed { message: String },
}

pub struct Envelope<T> {
    pub value: T,
}

impl Payload {
    pub const DEFAULT_ID: u64 = 0;

    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

impl<T> Envelope<T> {
    pub const DEFAULT_CAPACITY: usize = 4;

    pub fn new(value: T) -> Self {
        Self { value }
    }
}

pub fn parse_payload(input: &str) -> Result<Payload, FixtureError> {
    if input.is_empty() {
        Err(FixtureError::Invalid)
    } else {
        Ok(Payload {
            id: input.len() as u64,
            name: input.to_string(),
        })
    }
}
