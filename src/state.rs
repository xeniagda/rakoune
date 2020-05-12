use crate::gpu_primitives::Vertex;

pub struct State {
    pub verticies: Vec<Vertex>
}

impl State {
    pub fn new() -> State {
        let corn_tl = Vertex { position: [-0.5, 0.5], color: [1., 0., 0.] };
        let corn_tr = Vertex { position: [0.5, 0.5], color: [1., 0., 1.] };
        let corn_bl = Vertex { position: [-0.5, -0.5], color: [0., 1., 1.] };
        let corn_br = Vertex { position: [0.5, -0.5], color: [0., 0., 1.] };

        State {
            verticies: vec![
                corn_tl, corn_bl, corn_tr,
                corn_tr, corn_bl, corn_br,
            ]
        }
    }

    pub fn step(&mut self, dt: f32) {
        let angle_rot = dt;

        for vert in &mut self.verticies {
            let [x, y] = vert.position;
            let x_rot = x * angle_rot.cos() - y * angle_rot.sin();
            let y_rot = x * angle_rot.sin() + y * angle_rot.cos();

            vert.position[0] = x_rot;
            vert.position[1] = y_rot;
        }
    }
}
