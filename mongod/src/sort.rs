use bson::{Bson, Document};

use crate::field::Field;

/// The order in which to sort a field by.
#[derive(Clone, Serialize)]
pub enum Order<T>
where
    T: Field + Into<String>,
{
    /// Sort in ascending order, which is equivalent to `1`
    Asc,
    /// Sort in descending order, which is equivalent to `-1`
    Desc,
    /// Sort on a nested field
    Nested(Sort<T>),
}

impl<T> From<Order<T>> for Bson
where
    T: Field + Into<String>,
{
    fn from(order: Order<T>) -> Self {
        match order {
            // Mongo uses '1' and '-1' to denote sort order in the document
            // given to queries' '$sort' property
            Order::Asc => Bson::Int32(1),
            Order::Desc => Bson::Int32(-1),
            Order::Nested(s) => Bson::Document(s.into_document()),
        }
    }
}

/// A helper type to create a typed dictionary of sorted fields.
///
/// This type is used to sort documents fetched from the mongodb, it takes `Field`, `Order` pairs.
///
/// # Example
///
/// Sorting a user collection by name.
///
/// ```no_run
/// # async fn doc() -> Result<(), mongod::Error> {
/// use mongod::bson::Document;
/// use mongod::{AsField, Field, Order, Query, Sort};
/// use serde::{Deserialize, Serialize};
///
/// # use mongod_derive::{Bson, Mongo};
/// #[derive(Bson, Mongo, Deserialize, Serialize)]
/// #[mongo(collection="users", filter, update)]
/// pub struct User {
///     pub name: String,
/// }
///
/// impl AsField<UserField> for User {}
///
/// pub enum UserField {
///     Name,
/// }
///
/// impl Field for UserField {}
///
/// impl From<UserField> for String {
///     fn from(field: UserField) -> String {
///         match field {
///             UserField::Name => "name".to_owned(),
///         }
///     }
/// }
///
/// let client = mongod::Client::default();
///
/// let mut sort = Sort::new();
/// sort.push(UserField::Name, Order::Asc);
///
/// let _cursor = Query::find::<User>()
///     .sort(sort)
///     .query(&client)
///     .await
///     .unwrap();
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Serialize)]
pub struct Sort<T: Field + Into<String>>(Vec<(T, Order<T>)>);

impl<T> Default for Sort<T>
where
    T: Field + Into<String>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Field + Into<String>> Sort<T> {
    /// Creates an empty `Sort`.
    pub fn new() -> Self {
        Sort(vec![])
    }

    /// Pushes a (`Field`, `Order`) pair into the sort.
    pub fn push(&mut self, field: T, order: Order<T>) -> &mut Self {
        self.0.push((field, order));
        self
    }

    /// Converts the `Sort` into a BSON [`Document`](bson::Document).
    pub fn into_document(self) -> Document {
        let mut doc = Document::new();
        for (f, o) in self.0 {
            doc.insert(f, o);
        }
        doc
    }
}
