use std::marker::PhantomData;

use bson::Document;
use mongodb::options::{Collation, Hint, ReplaceOptions, WriteConcern};

use crate::collection::Collection;
use crate::filter::{AsFilter, Filter};
use crate::r#async::Client;

/// A querier to replace a document in a MongoDB collection.
///
/// # Examples
///
/// Replace a document in a collection.
///
/// ```no_run
/// # mod wrapper {
/// # use mongod_derive::{Bson, Mongo};
///
/// use mongod::{AsFilter, Comparator};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Bson, Mongo, Deserialize, Serialize)]
/// #[mongo(collection="users", field, filter, update)]
/// pub struct User {
///     name: String,
/// }
///
/// # async fn doc() -> Result<(), mongod::Error> {
/// let client = mongod::Client::new();
///
/// let mut filter = User::filter();
/// filter.name = Some(Comparator::Eq("foo".to_owned()));
///
/// let user = User {
///     name: "bar".to_owned(),
/// };
///
/// let oid = mongod::query::Replace::<User>::new()
///     .filter(filter)
///     .unwrap()
///     .query(&client, user)
///     .await
///     .unwrap();
/// println!("{:?}", oid);
/// # Ok(())
/// # }
/// # }
/// ```
#[derive(Clone)]
pub struct Replace<C: Collection> {
    filter: Option<Document>,
    options: ReplaceOptions,

    query_type: std::marker::PhantomData<C>,
}

impl<C: Collection> Default for Replace<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Collection> Replace<C> {
    /// Constructs a `Replace` querier.
    pub fn new() -> Self {
        Self {
            filter: None,
            options: ReplaceOptions::default(),

            query_type: PhantomData,
        }
    }

    /// Opt out of document-level validation.
    pub fn bypass_document_validation(mut self, enable: bool) -> Self {
        self.options.bypass_document_validation = Some(enable);
        self
    }

    /// The collation to use for the operation.
    ///
    /// Collation allows users to specify language-specific rules for string comparison, such as
    /// rules for lettercase and accent marks.
    pub fn collation(mut self, collation: Collation) -> Self {
        self.options.collation = Some(collation);
        self
    }

    /// The filter to use for the operation.
    ///
    /// # Errors
    ///
    /// This method errors if the filter could not be converted into a BSON `Document`.
    pub fn filter<F>(mut self, filter: F) -> crate::Result<Self>
    where
        C: AsFilter<F>,
        F: Filter,
    {
        self.filter = Some(filter.into_document()?);
        Ok(self)
    }

    /// A document or string that specifies the index to use to support the query predicate.
    pub fn hint(mut self, value: Hint) -> Self {
        self.options.hint = Some(value);
        self
    }

    /// Insert a document if no matching document is found.
    pub fn upsert(mut self, enable: bool) -> Self {
        self.options.upsert = Some(enable);
        self
    }

    /// The write concern for the operation.
    pub fn write_concern(mut self, concern: WriteConcern) -> Self {
        self.options.write_concern = Some(concern);
        self
    }

    /// Query the database with this querier.
    ///
    /// # Errors
    ///
    /// This method fails if:
    /// - the document could not be converted into a BSON `Document`.
    /// - the mongodb encountered an error.
    pub async fn query(self, client: &Client, document: C) -> crate::Result<bool> {
        let filter = match self.filter {
            Some(f) => f,
            None => Document::new(),
        };
        let result = client
            .database()
            .collection(C::COLLECTION)
            .replace_one(filter, document.into_document()?, self.options)
            .await
            .map_err(crate::error::mongodb)?;
        if result.modified_count > 0 {
            return Ok(true);
        }
        Ok(false)
    }

    /// Query the database with this querier in a blocking context.
    ///
    /// # Optional
    ///
    /// This requires the optional `blocking` feature to be enabled.
    ///
    /// # Errors
    ///
    /// This method fails if:
    /// - the document could not be converted into a BSON `Document`.
    /// - the mongodb encountered an error.
    #[cfg(feature = "blocking")]
    pub fn blocking(self, client: &crate::blocking::Client, document: C) -> crate::Result<bool> {
        let filter = match self.filter {
            Some(f) => f,
            None => bson::Document::new(),
        };
        let resp = client.execute(crate::blocking::Request::Replace(
            C::COLLECTION,
            filter,
            document.into_document()?,
            self.options,
        ))?;
        if let crate::blocking::Response::Replace(r) = resp {
            if r.modified_count > 0 {
                return Ok(true);
            }
            return Ok(false);
        }
        Err(crate::error::runtime(
            "incorrect response from blocking client",
        ))
    }
}
