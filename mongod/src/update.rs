use bson::Document;

use crate::error::Error;

/// Used to tie a type implementing [`Collection`](./trait.Collection.html) to its companion `Update` type.
///
/// # Examples
///
/// Tying `User` to its `Update`.
///
/// ```
/// # mod wrapper {
/// # use mongod_derive::{Bson, Mongo};
/// use mongod::bson::Document;
/// use mongod::{AsUpdate, Error, Update};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Bson, Mongo, Deserialize, Serialize)]
/// #[mongo(collection="users")]
/// pub struct User {
///     pub name: String,
/// }
///
/// #[derive(Default)]
/// pub struct UserUpdate {
///     pub name: Option<String>,
/// }
///
/// impl Update for UserUpdate {
///     fn new() -> Self {
///        UserUpdate::default()
///     }
///     fn into_document(self) -> Result<Document, Error> {
///         let mut doc = Document::new();
///         if let Some(value) = self.name {
///             doc.insert("name", value);
///         }
///         Ok(doc)
///     }
/// }
///
/// impl mongod::AsUpdate<UserUpdate> for User {
///     fn update() -> UserUpdate {
///         UserUpdate::default()
///     }
///     fn into_update(self) -> UserUpdate {
///         UserUpdate {
///             name: Some(self.name),
///         }
///     }
/// }
/// # }
/// ```
pub trait AsUpdate<U: Update> {
    /// Returns the `Collection`s update.
    fn update() -> U;
    /// Converts the `Collection` instance into its update.
    fn into_update(self) -> U;
}

/// Used to mark a type as an update for use in queries.
///
/// # Examples
///
/// Creating an update for user.
///
/// ```no_run
/// use mongod::bson::Document;
/// use mongod::{Error, Update};
///
/// #[derive(Default)]
/// pub struct UserUpdate {
///     pub name: Option<String>,
/// }
///
/// impl Update for UserUpdate {
///     fn new() -> Self {
///        UserUpdate::default()
///     }
///     fn into_document(self) -> Result<Document, Error> {
///         let mut doc = Document::new();
///         if let Some(value) = self.name {
///             doc.insert("name", value);
///         }
///         Ok(doc)
///     }
/// }
/// ```
pub trait Update {
    /// Constructs a new `Filter`.
    fn new() -> Self;
    /// Converts a `Filter` into a BSON `Document`.
    fn into_document(self) -> Result<Document, Error>;
}

/// Used for complex updates using MongoDB's update operators.
///
/// # NOTE
///
/// Not all operators are implemented yet...
///
/// # Examples
///
/// Unset the age of a user.
///
/// ```no_run
/// # mod wrapper {
/// # use mongod_derive::{Bson, Mongo};
/// use mongod::bson::Document;
/// use mongod::{AsFilter, AsUpdate, Comparator, Error, Update, Updates};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Bson, Mongo, Deserialize, Serialize)]
/// #[mongo(collection="users", field, filter)]
/// pub struct User {
///     pub name: String,
///     pub age: u32,
/// }
///
/// #[derive(Default)]
/// pub struct UserUpdate {
///     pub name: Option<String>,
///     pub age: Option<u32>,
/// }
///
/// impl Update for UserUpdate {
///     fn new() -> Self {
///        UserUpdate::default()
///     }
///     fn into_document(self) -> Result<Document, Error> {
///         let mut doc = Document::new();
///         if let Some(value) = self.name {
///             doc.insert("name", value);
///         }
///         if let Some(value) = self.age {
///             doc.insert("age", value);
///         }
///         Ok(doc)
///     }
/// }
///
/// impl AsUpdate<UserUpdate> for User {
///     fn update() -> UserUpdate {
///         UserUpdate::default()
///     }
///     fn into_update(self) -> UserUpdate {
///         UserUpdate {
///             name: Some(self.name),
///             age: Some(self.age),
///         }
///     }
/// }
///
/// # async fn doc() -> Result<(), mongod::Error> {
/// let client = mongod::Client::default();
///
/// let mut filter = User::filter();
/// filter.name = Some(Comparator::Eq("foo".to_owned()));
///
/// // NOTE: Unsetting a field that is not optional or doesn't have a default will cause
/// // deserialisation from the database to fail.
/// let unset = UserUpdate {
///     age: None,
///     ..UserUpdate::default()
/// };
/// let updates = Updates {
///     unset: Some(unset),
///     ..Updates::default()
/// };
///
/// let _cursor = client.update::<User, _, _>(filter, updates).await.unwrap();
/// # Ok(())
/// # }
/// # }
/// ```
// TODO: Implement the other update operators: https://docs.mongodb.com/manual/reference/operator/update/#id1
#[derive(Default)]
pub struct Updates<U: Update> {
    /// Sets the value of a field in a document.
    pub set: Option<U>,
    /// Removes the specified field from a document.
    pub unset: Option<U>,
}

impl<U: Update> Updates<U> {
    /// Convert `Updates` into a BSON `Document`.
    pub fn into_document(self) -> Result<Document, Error> {
        let mut document = crate::bson::Document::new();
        if let Some(set) = self.set {
            document.insert("$set", set.into_document()?);
        }
        if let Some(unset) = self.unset {
            document.insert("$unset", unset.into_document()?);
        }
        Ok(document)
    }
}

impl<U: Default + Update> Update for Updates<U> {
    fn new() -> Self {
        Updates::default()
    }
    fn into_document(self) -> Result<Document, Error> {
        self.into_document()
    }
}
