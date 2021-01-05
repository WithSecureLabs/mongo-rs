//! The query operations that can be perfomed on a MongoDB.
use crate::collection::Collection;

mod delete;
mod find;
mod insert;
mod replace;
mod update;

pub use self::delete::Delete;
pub use self::find::Find;
pub use self::insert::Insert;
pub use self::replace::Replace;
pub use self::update::Update;

/// A convenience wrapper for easy access to queriers.
pub struct Query;

impl Query {
    /// Returns a `Delete` querier.
    pub fn delete<C>() -> Delete<C>
    where
        C: Collection,
    {
        Delete::new()
    }

    /// Returns a `Find` querier.
    pub fn find<C>() -> Find<C>
    where
        C: Collection,
    {
        Find::new()
    }

    /// Returns a `Insert` querier.
    pub fn insert<C>() -> Insert<C>
    where
        C: Collection,
    {
        Insert::new()
    }

    /// Returns a `Replace`
    pub fn replace<C>() -> Replace<C>
    where
        C: Collection,
    {
        Replace::new()
    }

    /// Returns a `Update`
    pub fn update<C>() -> Update<C>
    where
        C: Collection,
    {
        Update::new()
    }
}
