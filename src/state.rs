pub struct State {
    pub content: String,
}

impl State {
    pub fn new() -> State {
        State {
            content: "ni li ilo pi\npana sitelen".to_string(),
        }
    }

    pub fn step(&mut self, dt: f32) {
    }
}
