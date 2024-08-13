#[derive(Copy, Clone, Debug)]
pub enum Model {
    #[allow(unused)]
    Gpt35,
    #[allow(unused)]
    Gpt4,
    #[allow(unused)]
    Gpt4o,
}

impl Model {
    pub fn name(&self) -> &'static str {
        match self {
            Model::Gpt35 => "gpt-3.5-turbo",
            Model::Gpt4 => "gpt-4",
            Model::Gpt4o => "gpt-4o",
        }
    }
}
