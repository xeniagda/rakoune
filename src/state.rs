pub struct State {
    content: String,
}

impl State {
    pub fn new() -> State {
        State {
            content: "ni li pona tawa mi a".to_string(),
        }
    }

    pub fn step(&mut self, dt: f32) {
    }
}
