use std::marker::PhantomData;

use mongodb::options::{Collation, DeleteOptions, Hint, WriteConcern};

use crate::collection::Collection;
use crate::filter::{AsFilter, Filter};
use crate::r#async::Client;
use bson::Document;

/// A querier to delete documents from a MongoDB collection.
///
/// # Examples
///
/// Delete all documents from a collection.
///
/// ```no_run
/// # async fn doc() -> Result<(), mongod::Error> {
/// # use mongod_derive::{Bson, Mongo};
///
/// #[derive(Bson, Mongo)]
/// #[mongo(collection="users", field, filter, update)]
/// pub struct User {
///     name: String,
/// }
///
/// let client = mongod::Client::new();
///
/// let deleted = mongod::query::Delete::<User>::new()
///     .query(&client)
///     .await
///     .unwrap();
///
/// println!("delete {} documents", deleted);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Delete<C: Collection> {
    filter: Option<bson::Document>,
    many: bool,
    options: DeleteOptions,

    query_type: std::marker::PhantomData<C>,
}

impl<C: Collection> Default for Delete<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Collection> Delete<C> {
    /// Constructs a `Delete` querier.
    pub fn new() -> Self {
        Self {
            filter: None,
            many: true,
            options: DeleteOptions::default(),

            query_type: PhantomData,
        }
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
    pub fn hint(mut self, hint: Hint) -> Self {
        self.options.hint = Some(hint);
        self
    }

    /// Enable delete many for this operation.
    ///
    /// Removes all documents that match the filter from a collection.
    pub fn many(mut self, many: bool) -> Self {
        self.many = many;
        self
    }

    /// The write concern for the operation.
    pub fn write_concern(mut self, write_concern: WriteConcern) -> Self {
        self.options.write_concern = Some(write_concern);
        self
    }

    /// Query the database with this querier.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub async fn query(self, client: &Client) -> crate::Result<u64> {
        let filter = match self.filter {
            Some(f) => f,
            None => bson::Document::new(),
        };
        let result = if self.many {
            client
                .database()
                .collection::<Document>(C::COLLECTION)
                .delete_many(filter, Some(self.options))
                .await
        } else {
            client
                .database()
                .collection::<Document>(C::COLLECTION)
                .delete_one(filter, Some(self.options))
                .await
        }
        .map_err(crate::error::mongodb)?;
        Ok(result.deleted_count)
    }

    /// Query the database with this querier in a blocking context.
    ///
    /// # Optional
    ///
    /// This requires the optional `blocking` feature to be enabled.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    #[cfg(feature = "blocking")]
    pub fn blocking(self, client: &crate::blocking::Client) -> crate::Result<u64> {
        let filter = match self.filter {
            Some(f) => f,
            None => bson::Document::new(),
        };
        let resp = client.execute(crate::blocking::Request::Delete(
            self.many,
            C::COLLECTION,
            filter,
            self.options,
        ))?;
        if let crate::blocking::Response::Delete(r) = resp {
            return Ok(r.deleted_count);
        }
        Err(crate::error::runtime(
            "incorrect response from blocking client",
        ))
    }
}
