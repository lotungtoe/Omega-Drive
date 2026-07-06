use rusqlite::Connection;

pub trait DbExecutor: Send + Sync {
    fn read(&self, f: &mut dyn FnMut(&Connection));
    fn write(&self, f: &mut dyn FnMut(&Connection));
}
