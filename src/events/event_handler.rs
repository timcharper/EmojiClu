pub trait EventHandler<T> {
    fn handle_event(&mut self, event: &T);
}
