#[derive(Debug, Clone, Copy)]
pub(crate) struct Rect {
    pub pos: (u32, u32),
    pub size: (u32, u32),
}

impl Rect {
    pub fn intersects(&self, other: &Rect) -> bool {
        let self_max = self.max();
        let other_max = other.max();

        let x_intersect = self.pos.0 < other_max.0 && self_max.0 > other.pos.0;
        let y_intersect = self.pos.1 < other_max.1 && self_max.1 > other.pos.1;

        x_intersect && y_intersect
    }

    pub fn max(&self) -> (u32, u32) {
        (self.pos.0 + self.size.0, self.pos.1 + self.size.1)
    }
}
