use std::marker::Unpin;

//use bson::Document;
use serde::{de::DeserializeOwned, ser::Serialize};

//use crate::error::Error;

/// Used to create a mongo collection from a type.
///
/// This trait can be thought of as a collection's name along with its schema.
///
/// This trait is required in order to intract with the mongo [`Client`](`crate::Client`). The
/// implementation is used to tie a collection name to the chosen type while also defining how to
/// convert it serde [`Document`](`struct@bson::Document`).
///
/// # Examples
///
/// Defining a struct as a mongo document.
///
/// ```
/// use std::convert::TryFrom;
///
/// use mongod::{Collection};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Deserialize, Serialize)]
/// pub struct User {
///     pub name: String,
/// }
///
/// impl Collection for User {
///     const COLLECTION: &'static str = "users";
/// }
/// ```
pub trait Collection: DeserializeOwned + Serialize + Unpin + Send + Sync {
    /// The name of the collection to store the documents in.
    const COLLECTION: &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde::{Deserialize, Serialize};

    use crate::bson::{self, Document};

    #[derive(Deserialize, Serialize)]
    struct User {
        name: String,
    }

    impl Collection for User {
        const COLLECTION: &'static str = "users";
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
        let b = bson::to_bson(&user).unwrap();
        let doc = b.as_document().unwrap();
        assert_eq!(doc.get("name").unwrap().as_str().unwrap(), "foo".to_owned());
    }

    #[test]
    fn bson_to_document() {
        let mut doc = Document::new();
        doc.insert("name", "foo".to_owned());
        let b = bson::Bson::Document(doc);
        let user: User = bson::from_bson(b).unwrap();
        assert_eq!(user.name, "foo".to_owned());
    }
}
