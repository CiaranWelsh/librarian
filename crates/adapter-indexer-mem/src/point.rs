use librarian_domain::{Chunk, Vector};

#[derive(Debug, Clone)]
pub struct Point {
    pub chunk: Chunk,
    pub vector: Vector,
}
