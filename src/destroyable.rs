// trait implemented when circular references will not be auto-cleaned up and we need to manually break the chain
pub trait Destroyable {
    fn destroy(&mut self);
}
