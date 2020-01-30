#[derive(Debug, Clone, Copy)]
pub(crate) struct Aabb {
    pub pos: (u32, u32),
    pub size: (u32, u32),
}

impl Aabb {
    pub fn intersects(&self, other: &Aabb) -> bool {
        let a_center = (
            (self.pos.0 + self.size.0) as i32,
            (self.pos.1 + self.size.1) as i32,
        );
        let b_center = (
            (other.pos.0 + other.size.0) as i32,
            (other.pos.1 + other.size.1) as i32,
        );

        let size_avg = (
            (self.size.0 + other.size.0) as i32 / 2,
            (self.size.1 + other.size.1) as i32 / 2,
        );

        let x_overlap = (b_center.0 - a_center.0).abs() < size_avg.0;
        let y_overlap = (b_center.1 - a_center.1).abs() < size_avg.1;

        x_overlap && y_overlap
    }
}
