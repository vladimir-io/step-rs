use steprs_schema::Vec3;

/// Orthonormal basis on a planar face (u × v = normal).
#[derive(Debug, Clone, Copy)]
pub struct PlaneFrame {
    pub origin: Vec3,
    pub u: Vec3,
    pub v: Vec3,
}

impl PlaneFrame {
    pub fn from_face(origin: Vec3, normal: Vec3, ref_dir: Option<Vec3>) -> Self {
        let n = normal.normalize();
        let u = ref_dir
            .map(|r| {
                let proj = r.sub(n.scale(r.dot(n)));
                if proj.dot(proj) > 1e-12 {
                    proj.normalize()
                } else {
                    fallback_u(n)
                }
            })
            .unwrap_or_else(|| fallback_u(n));
        let v = n.cross(u).normalize();
        Self { origin, u, v }
    }

    pub fn to_world(&self, u: f64, v: f64) -> [f64; 3] {
        let p = self
            .origin
            .add(self.u.scale(u))
            .add(self.v.scale(v));
        [p.x, p.y, p.z]
    }

    pub fn project(&self, p: Vec3) -> (f64, f64) {
        let d = p.sub(self.origin);
        (d.dot(self.u), d.dot(self.v))
    }

    pub fn project_array(&self, p: [f64; 3]) -> (f64, f64) {
        self.project(Vec3::new(p[0], p[1], p[2]))
    }
}

fn fallback_u(n: Vec3) -> Vec3 {
    if n.z.abs() < 0.9 {
        Vec3::new(0.0, 0.0, 1.0).cross(n).normalize()
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    }
}

/// Signed area of a closed polygon projected on the plane (mm²).
pub fn loop_area_uv(points: &[[f64; 3]], frame: &PlaneFrame) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    let uv: Vec<_> = points
        .iter()
        .map(|p| frame.project_array(*p))
        .collect();
    for i in 0..uv.len() {
        let j = (i + 1) % uv.len();
        area += uv[i].0 * uv[j].1 - uv[j].0 * uv[i].1;
    }
    area * 0.5
}

pub fn bbox_uv(points: &[[f64; 3]], frame: &PlaneFrame) -> (f64, f64, f64, f64) {
    let mut min_u = f64::INFINITY;
    let mut max_u = f64::NEG_INFINITY;
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for p in points {
        let (u, v) = frame.project_array(*p);
        min_u = min_u.min(u);
        max_u = max_u.max(u);
        min_v = min_v.min(v);
        max_v = max_v.max(v);
    }
    (min_u, max_u, min_v, max_v)
}

pub fn rect_corners_uv(
    frame: &PlaneFrame,
    min_u: f64,
    max_u: f64,
    min_v: f64,
    max_v: f64,
) -> Vec<[f64; 3]> {
    vec![
        frame.to_world(min_u, min_v),
        frame.to_world(max_u, min_v),
        frame.to_world(max_u, max_v),
        frame.to_world(min_u, max_v),
    ]
}
