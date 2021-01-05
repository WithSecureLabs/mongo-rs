use std::convert::{TryFrom, TryInto};

use bson::{Bson, Document};

use crate::error::Error;
use crate::ext;

/// The BSON comparators for comparison of different BSON type values
pub enum Comparator<T>
where
    T: TryInto<ext::bson::Bson>,
    T::Error: Into<ext::bson::ser::Error>,
{
    /// Matches values that are equal to a specified value.
    Eq(T),
    /// Matches values that are greater than a specified value.
    Gt(T),
    /// Matches values that are greater than or equal to a specified value.
    Gte(T),
    /// Matches any of the values specified in an array.
    In(Vec<T>),
    /// Matches values that are less than a specified value.
    Lt(T),
    /// Matches values that are less than or equal to a specified value.
    Lte(T),
    /// Matches all values that are not equal to a specified value.
    Ne(T),
    /// Matches none of the values specified in an array.
    Nin(Vec<T>),
}

impl<T> TryFrom<Comparator<T>> for Bson
where
    T: TryInto<ext::bson::Bson>,
    T::Error: Into<ext::bson::ser::Error>,
{
    type Error = ext::bson::ser::Error;
    fn try_from(value: Comparator<T>) -> Result<Self, Self::Error> {
        Ok(match value {
            Comparator::Eq(t) => bson!({ "$eq": t.try_into().map_err(|e| e.into())?.0 }),
            Comparator::Gt(t) => bson!({ "$gt": t.try_into().map_err(|e| e.into())?.0 }),
            Comparator::Gte(t) => bson!({ "$gte": t.try_into().map_err(|e| e.into())?.0 }),
            Comparator::In(t) => {
                let int = t
                    .into_iter()
                    .map(|t| t.try_into())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.into())?;
                bson!({ "$in": Bson::Array(int.into_iter().map(|b| b.0).collect()) })
            }
            Comparator::Lt(t) => bson!({ "$lt": t.try_into().map_err(|e| e.into())?.0 }),
            Comparator::Lte(t) => bson!({ "$lte": t.try_into().map_err(|e| e.into())?.0 }),
            Comparator::Ne(t) => bson!({ "$ne": t.try_into().map_err(|e| e.into())?.0 }),
            Comparator::Nin(t) => {
                let int = t
                    .into_iter()
                    .map(|t| t.try_into())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.into())?;
                bson!({ "$nin": Bson::Array(int.into_iter().map(|b| b.0).collect()) })
            }
        })
    }
}

impl<T> TryFrom<Comparator<T>> for ext::bson::Bson
where
    T: TryInto<ext::bson::Bson>,
    T::Error: Into<ext::bson::ser::Error>,
{
    type Error = ext::bson::ser::Error;
    fn try_from(value: Comparator<T>) -> Result<Self, Self::Error> {
        Ok(ext::bson::Bson(Bson::try_from(value)?))
    }
}

/// Used to tie a type implementing [`Collection`](./trait.Collection.html) to its companion `Filter` type.
///
/// # Examples
///
/// Tying `User` to its `Filter`.
///
/// ```
/// use std::convert::TryFrom;
///
/// # use mongo_derive::{Bson, Mongo};
/// use mongo::bson::Document;
/// use mongo::{AsFilter, Filter, Comparator, Error};
/// use mongo::ext::bson::Bson;
///
/// #[derive(Bson, Mongo)]
/// #[mongo(collection="users")]
/// pub struct User {
///     pub name: String,
/// }
///
/// #[derive(Default)]
/// pub struct UserFilter {
///     pub name: Option<Comparator<String>>,
/// }
///
/// impl Filter for UserFilter {
///     fn new() -> Self {
///         Self::default()
///     }
///
///     fn into_document(self) -> Result<Document, Error> {
///         let mut doc = Document::new();
///         if let Some(value) = self.name {
///             doc.insert("name", Bson::try_from(value)?.0);
///         }
///         Ok(doc)
///     }
/// }
///
/// impl AsFilter<UserFilter> for User {
///     fn filter() -> UserFilter {
///         UserFilter::default()
///     }
///
///     fn into_filter(self) -> UserFilter {
///         UserFilter {
///             name: Some(Comparator::Eq(self.name)),
///         }
///     }
/// }
/// ```
pub trait AsFilter<T: Filter> {
    /// Returns the `Collection`s filter.
    fn filter() -> T;
    /// Converts the `Collection` instance into its filter.
    fn into_filter(self) -> T;
}

/// Used to mark a type as a filter for use in queries.
///
/// # Examples
///
/// Creating a filter for user.
///
/// ```
/// use std::convert::TryFrom;
///
/// use mongo::bson::Document;
/// use mongo::{Filter, Comparator, Error};
/// use mongo::ext::bson::Bson;
///
/// #[derive(Default)]
/// pub struct UserFilter {
///     name: Option<Comparator<String>>,
/// }
///
/// impl Filter for UserFilter {
///     fn new() -> Self {
///         Self::default()
///     }
///
///     fn into_document(self) -> Result<Document, Error> {
///         let mut doc = Document::new();
///         if let Some(value) = self.name {
///             doc.insert("name", Bson::try_from(value)?.0);
///         }
///         Ok(doc)
///     }
/// }
/// ```
pub trait Filter {
    /// Constructs a new `Filter`.
    fn new() -> Self;
    /// Converts a `Filter` into a BSON `Document`.
    fn into_document(self) -> Result<Document, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ext;

    pub struct User {
        pub name: String,
    }

    #[derive(Default)]
    pub struct UserFilter {
        name: Option<Comparator<String>>,
    }

    impl Filter for UserFilter {
        fn new() -> Self {
            Self::default()
        }

        fn into_document(self) -> crate::Result<Document> {
            let mut doc = Document::new();
            if let Some(value) = self.name {
                doc.insert("name", ext::bson::Bson::try_from(value)?.0);
            }
            Ok(doc)
        }
    }

    impl AsFilter<UserFilter> for User {
        fn filter() -> UserFilter {
            UserFilter::default()
        }

        fn into_filter(self) -> UserFilter {
            UserFilter {
                name: Some(Comparator::Eq(self.name)),
            }
        }
    }

    #[test]
    fn user_into_filter() {
        let user = User {
            name: "foo".to_owned(),
        };
        let f = user.into_filter();
        let name = if let Some(Comparator::Eq(val)) = f.name {
            val
        } else {
            "".to_owned()
        };
        assert_eq!(name, "foo".to_owned());
    }

    #[test]
    fn filter_into_document() {
        let filter = UserFilter {
            name: Some(Comparator::Eq("foo".to_owned())),
        };
        let doc = filter.into_document().unwrap();
        assert_eq!(
            doc.get("name")
                .unwrap()
                .as_document()
                .unwrap()
                .get("$eq")
                .unwrap()
                .as_str()
                .unwrap(),
            "foo".to_owned()
        );
    }
}
