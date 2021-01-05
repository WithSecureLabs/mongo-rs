use bson::Document;

use crate::error::Error;

/// Used to create a mongo collection from a type.
///
/// This trait can be thought of as a collection's name along with its schema.
///
/// This trait is required in order to intract with the mongo [`Client`](`crate::Client`). The
/// implementation is used to tie a collection name to the chosen type while also defining how to
/// convert to and from a BSON [`Document`](`struct@bson::Document`).
///
/// # Examples
///
/// Defining a struct as a mongo document.
///
/// ```
/// use std::convert::TryFrom;
///
/// use mongo::bson::{self, Document};
/// use mongo::{Collection, Error};
/// use mongo::ext;
///
/// pub struct User {
///     pub name: String,
/// }
///
/// impl Collection for User {
///     const COLLECTION: &'static str = "users";
///
///     fn from_document(document: Document) -> Result<Self, Error> {
///         let mut document = document;
///         let mut name: Option<String> = None;
///         if let Some(value) = document.remove("name") {
///             name = Some(String::try_from(ext::bson::Bson(value))?);
///         }
///         if name.is_none() {
///            return Err(Error::invalid_document("missing required fields"));
///         }
///         Ok(Self {
///             name: name.expect("could not get name"),
///         })
///     }
///
///     fn into_document(self) -> Result<Document, Error> {
///         let mut doc = Document::new();
///         doc.insert("name", self.name);
///         Ok(doc)
///     }
/// }
/// ```
pub trait Collection {
    /// The name of the collection to store the documents in.
    const COLLECTION: &'static str;

    /// Convert from a BSON `Document` into the `Collection`s type.
    fn from_document(document: Document) -> Result<Self, Error>
    where
        Self: Sized;
    /// Convert the `Collection`s type into a BSON `Document`.
    fn into_document(self) -> Result<Document, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::bson::{self, Document};
    use crate::Error as MongoError;

    struct User {
        name: String,
    }

    impl Collection for User {
        const COLLECTION: &'static str = "users";

        fn from_document(document: Document) -> Result<Self, MongoError> {
            let mut document = document;
            let mut name: Option<String> = None;
            if let Some(value) = document.remove("name") {
                name = bson::from_bson(value).map_err(MongoError::invalid_document)?;
            }
            if name.is_none() {
                return Err(MongoError::invalid_document("missing required fields"));
            }
            Ok(User {
                name: name.expect("could not get name"),
            })
        }

        fn into_document(self) -> Result<Document, Error> {
            let mut doc = Document::new();
            doc.insert("name", self.name);
            Ok(doc)
        }
    }

    #[test]
    fn collection() {
        assert_eq!(User::COLLECTION, "users");
    }

    #[test]
    fn document_to_bson() {
        let user = User {
            name: "foo".to_owned(),
        };
        let doc = user.into_document().unwrap();
        assert_eq!(doc.get("name").unwrap().as_str().unwrap(), "foo".to_owned());
    }

    #[test]
    fn bson_to_document() {
        let mut doc = Document::new();
        doc.insert("name", "foo".to_owned());
        let user = User::from_document(doc).unwrap();
        assert_eq!(user.name, "foo".to_owned());
    }
}
