//! Low-poly mesh builder: flat-shaded triangles with per-vertex color.
//! Normals are always oriented away from a supplied interior point so winding
//! never matters (the renderer draws without back-face culling).
use crate::math::*;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    /// rgb + a: a=1 lit, a=0 unlit/emissive
    pub col: [f32; 4],
}

pub type Col = [f32; 4];
#[inline]
pub fn lit(c: [f32; 3]) -> Col {
    [c[0], c[1], c[2], 1.0]
}
#[inline]
pub fn unlit(c: [f32; 3]) -> Col {
    [c[0], c[1], c[2], 0.0]
}
pub fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = clampf(t, 0.0, 1.0);
    [lerpf(a[0], b[0], t), lerpf(a[1], b[1], t), lerpf(a[2], b[2], t)]
}
pub fn scale3(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub struct Mesh {
    pub v: Vec<Vertex>,
}

impl Mesh {
    pub fn new() -> Mesh {
        Mesh { v: Vec::with_capacity(60000) }
    }
    pub fn clear(&mut self) {
        self.v.clear();
    }
    #[inline]
    pub fn tri_n(&mut self, a: V3, b: V3, c: V3, n: V3, col: Col) {
        let n = n.arr();
        self.v.push(Vertex { pos: a.arr(), nrm: n, col });
        self.v.push(Vertex { pos: b.arr(), nrm: n, col });
        self.v.push(Vertex { pos: c.arr(), nrm: n, col });
    }
    /// Triangle with normal oriented away from `inside`.
    #[inline]
    pub fn tri(&mut self, a: V3, b: V3, c: V3, inside: V3, col: Col) {
        let mut n = (b - a).cross(c - a).norm();
        let cen = (a + b + c) * (1.0 / 3.0);
        if n.dot(cen - inside) < 0.0 {
            n = -n;
        }
        self.tri_n(a, b, c, n, col);
    }
    pub fn quad(&mut self, a: V3, b: V3, c: V3, d: V3, inside: V3, col: Col) {
        self.tri(a, b, c, inside, col);
        self.tri(a, c, d, inside, col);
    }
    pub fn quad_n(&mut self, a: V3, b: V3, c: V3, d: V3, n: V3, col: Col) {
        self.tri_n(a, b, c, n, col);
        self.tri_n(a, c, d, n, col);
    }

    /// Tapered prism between a and b.
    pub fn prism(&mut self, a: V3, b: V3, ra: f32, rb: f32, sides: usize, ref_up: V3, col: Col) {
        let axis = (b - a).norm_or(UP);
        let mut x = ref_up - axis * ref_up.dot(axis);
        if x.len2() < 1e-6 {
            x = axis.cross(v3(1.0, 0.0, 0.0));
            if x.len2() < 1e-6 {
                x = axis.cross(v3(0.0, 0.0, 1.0));
            }
        }
        let x = x.norm();
        let y = axis.cross(x);
        let center = (a + b) * 0.5;
        let mut ra_pts = [ZERO; 8];
        let mut rb_pts = [ZERO; 8];
        let n = sides.min(8);
        for i in 0..n {
            let ang = i as f32 / n as f32 * std::f32::consts::TAU;
            let d = x * ang.cos() + y * ang.sin();
            ra_pts[i] = a + d * ra;
            rb_pts[i] = b + d * rb;
        }
        for i in 0..n {
            let j = (i + 1) % n;
            self.quad(ra_pts[i], ra_pts[j], rb_pts[j], rb_pts[i], center, col);
        }
        for i in 1..n - 1 {
            self.tri(ra_pts[0], ra_pts[i], ra_pts[i + 1], center, col);
            self.tri(rb_pts[0], rb_pts[i], rb_pts[i + 1], center, col);
        }
    }

    /// Oriented box: center, frame axes (f,u,r), half extents along (f,u,r).
    pub fn obox(&mut self, c: V3, f: V3, u: V3, r: V3, he: V3, col: Col) {
        let fx = f * he.x;
        let uy = u * he.y;
        let rz = r * he.z;
        let p = |sx: f32, sy: f32, sz: f32| c + fx * sx + uy * sy + rz * sz;
        let corners = [p(-1., -1., -1.), p(1., -1., -1.), p(1., 1., -1.), p(-1., 1., -1.), p(-1., -1., 1.), p(1., -1., 1.), p(1., 1., 1.), p(-1., 1., 1.)];
        const F: [[usize; 4]; 6] = [[0, 1, 2, 3], [4, 5, 6, 7], [0, 1, 5, 4], [3, 2, 6, 7], [0, 3, 7, 4], [1, 2, 6, 5]];
        for q in F {
            self.quad(corners[q[0]], corners[q[1]], corners[q[2]], corners[q[3]], c, col);
        }
    }

    /// Connect two rings of equal size (closed loops).
    pub fn ring_strip(&mut self, a: &[V3], b: &[V3], inside: V3, col: Col) {
        let n = a.len();
        for i in 0..n {
            let j = (i + 1) % n;
            self.quad(a[i], a[j], b[j], b[i], inside, col);
        }
    }
    pub fn cap(&mut self, ring: &[V3], inside: V3, col: Col) {
        for i in 1..ring.len() - 1 {
            self.tri(ring[0], ring[i], ring[i + 1], inside, col);
        }
    }

    /// Flat polygon on the ground (normal up).
    pub fn disc(&mut self, c: V3, r: f32, n: usize, col: Col) {
        let mut prev = c + v3(r, 0.0, 0.0);
        for i in 1..=n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let p = c + v3(a.cos() * r, 0.0, a.sin() * r);
            self.tri_n(c, prev, p, UP, col);
            prev = p;
        }
    }

    /// Octahedron (droplets, dust).
    pub fn octa(&mut self, c: V3, s: f32, stretch: V3, col: Col) {
        let ax = stretch.norm_or(UP);
        let mut x = ax.cross(UP);
        if x.len2() < 1e-4 {
            x = ax.cross(v3(1.0, 0.0, 0.0));
        }
        let x = x.norm();
        let y = ax.cross(x);
        let el = 1.0 + stretch.len() * 0.06;
        let pts = [c + ax * (s * el), c - ax * (s * el), c + x * s, c - x * s, c + y * s, c - y * s];
        for &(i, j) in &[(2, 4), (4, 3), (3, 5), (5, 2)] {
            self.tri(pts[0], pts[i], pts[j], c, col);
            self.tri(pts[1], pts[i], pts[j], c, col);
        }
    }

    /// Ellipsoid from a subdivided icosahedron in a local frame.
    /// `paint` returns a color per face (by local unit-sphere face center) or None to skip.
    pub fn ico<F: FnMut(V3) -> Option<Col>>(&mut self, fr: &Frame, scale: V3, mut paint: F) {
        let ico = ico_data();
        let c = fr.o;
        let tx = |l: V3| c + fr.f * (l.x * scale.x) + fr.u * (l.y * scale.y) + fr.r * (l.z * scale.z);
        for f in &ico.faces {
            let (a, b, cc) = (ico.verts[f[0]], ico.verts[f[1]], ico.verts[f[2]]);
            let cen = (a + b + cc).norm();
            if let Some(col) = paint(cen) {
                self.tri(tx(a), tx(b), tx(cc), c, col);
            }
        }
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}

pub struct IcoData {
    pub verts: Vec<V3>,
    pub faces: Vec<[usize; 3]>,
}

/// Icosahedron subdivided once (80 faces). Local coords are (f, u, r).
pub fn ico_data() -> &'static IcoData {
    static D: OnceLock<IcoData> = OnceLock::new();
    D.get_or_init(|| {
        let t = (1.0 + 5f32.sqrt()) / 2.0;
        let mut verts: Vec<V3> = [
            v3(-1.0, t, 0.0), v3(1.0, t, 0.0), v3(-1.0, -t, 0.0), v3(1.0, -t, 0.0),
            v3(0.0, -1.0, t), v3(0.0, 1.0, t), v3(0.0, -1.0, -t), v3(0.0, 1.0, -t),
            v3(t, 0.0, -1.0), v3(t, 0.0, 1.0), v3(-t, 0.0, -1.0), v3(-t, 0.0, 1.0),
        ]
        .iter()
        .map(|v| v.norm())
        .collect();
        let faces0: [[usize; 3]; 20] = [
            [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11], [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
            [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9], [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
        ];
        let mut cache = std::collections::HashMap::new();
        let mut mid = |a: usize, b: usize, verts: &mut Vec<V3>| -> usize {
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&i) = cache.get(&key) {
                return i;
            }
            let m = ((verts[a] + verts[b]) * 0.5).norm();
            verts.push(m);
            let i = verts.len() - 1;
            cache.insert(key, i);
            i
        };
        let mut faces = Vec::new();
        for f in faces0 {
            let a = mid(f[0], f[1], &mut verts);
            let b = mid(f[1], f[2], &mut verts);
            let c = mid(f[2], f[0], &mut verts);
            faces.push([f[0], a, c]);
            faces.push([f[1], b, a]);
            faces.push([f[2], c, b]);
            faces.push([a, b, c]);
        }
        IcoData { verts, faces }
    })
}
