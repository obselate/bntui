use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn rotate_x(self, a: f32) -> Self {
        let (s, c) = (a.sin(), a.cos());
        Self {
            x: self.x,
            y: self.y * c - self.z * s,
            z: self.y * s + self.z * c,
        }
    }
    fn rotate_y(self, a: f32) -> Self {
        let (s, c) = (a.sin(), a.cos());
        Self {
            x: self.x * c + self.z * s,
            y: self.y,
            z: -self.x * s + self.z * c,
        }
    }
    fn rotate_z(self, a: f32) -> Self {
        let (s, c) = (a.sin(), a.cos());
        Self {
            x: self.x * c - self.y * s,
            y: self.x * s + self.y * c,
            z: self.z,
        }
    }
}

// 8 corners of a cube
const VERTS: [Vec3; 8] = [
    Vec3 {
        x: -1.0,
        y: -1.0,
        z: -1.0,
    },
    Vec3 {
        x: 1.0,
        y: -1.0,
        z: -1.0,
    },
    Vec3 {
        x: 1.0,
        y: 1.0,
        z: -1.0,
    },
    Vec3 {
        x: -1.0,
        y: 1.0,
        z: -1.0,
    },
    Vec3 {
        x: -1.0,
        y: -1.0,
        z: 1.0,
    },
    Vec3 {
        x: 1.0,
        y: -1.0,
        z: 1.0,
    },
    Vec3 {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    },
    Vec3 {
        x: -1.0,
        y: 1.0,
        z: 1.0,
    },
];

// 12 edges connecting the corners
const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0), // front face
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4), // back face
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7), // connectors
];

pub struct SpinCube {
    pub angle_x: f32,
    pub angle_y: f32,
    pub angle_z: f32,
    pub color: Color,
    pub frozen: bool,
}

impl SpinCube {
    pub fn new() -> Self {
        Self {
            angle_x: 0.6,   // ~35 degrees - classic isometric tilt
            angle_y: 0.785, // ~45 degrees - corner-on view
            angle_z: 0.0,
            color: Color::White,
            frozen: true,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.angle_y += 0.7 * dt;
    }
}

impl Widget for &mut SpinCube {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let w = area.width as f32;
        let h = area.height as f32;
        if w < 4.0 || h < 4.0 {
            return;
        }

        let cx = w / 2.0;
        let cy = h / 2.0;
        let scale = h.min(w * 0.5) * 1.20;
        let distance = 6.0;
        let style = Style::default().fg(self.color);

        // rotate and project all 8 vertices to 2D
        let projected: Vec<Option<(f32, f32)>> = VERTS
            .iter()
            .map(|v| {
                let r = v
                    .rotate_y(self.angle_y)
                    .rotate_x(self.angle_x)
                    .rotate_z(self.angle_z);
                let z = r.z + distance;
                if z < 0.1 {
                    return None;
                }
                let inv_z = 1.0 / z;
                Some((
                    cx + r.x * scale * inv_z * 2.0, // *2 because terminal chars are tall
                    cy + r.y * scale * inv_z,
                ))
            })
            .collect();

        // draw each edge by interpolating points along it
        for (i0, i1) in EDGES {
            if let (Some((x0, y0)), Some((x1, y1))) = (projected[i0], projected[i1]) {
                for s in 0..=60 {
                    let t = s as f32 / 60.0;
                    let px = (x0 + (x1 - x0) * t) as u16;
                    let py = (y0 + (y1 - y0) * t) as u16;
                    if px < area.width && py < area.height {
                        buf[(area.x + px, area.y + py)]
                            .set_char('$')
                            .set_style(style);
                    }
                }
            }
        }
    }
}
