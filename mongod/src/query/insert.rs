use std::collections::HashMap;
use std::marker::PhantomData;

use bson::{Bson, Document};
use mongodb::options::{InsertManyOptions, WriteConcern};

use crate::collection::Collection;
use crate::r#async::Client;

/// A querier to insert documents into a MongoDB collection.
///
/// # Examples
///
/// Insert a document into a collection.
///
/// ```no_run
/// # mod wrapper {
/// # use mongod_derive::{Bson, Mongo};
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
/// let user = User { name: "foo".to_owned() };
///
/// let result = mongod::query::Insert::<User>::new()
///     .query(&client, vec![user])
///     .await
///     .unwrap();
///
/// println!("(index: oid) {:?}", result);
/// # Ok(())
/// # }
/// # }
/// ```
#[derive(Clone)]
pub struct Insert<C: Collection> {
    options: InsertManyOptions,

    query_type: std::marker::PhantomData<C>,
}

impl<C: Collection> Default for Insert<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Collection> Insert<C> {
    /// Constructs a `Insert` querier.
    pub fn new() -> Self {
        Self {
            options: InsertManyOptions::default(),

            query_type: PhantomData,
        }
    }

    /// Opt out of document-level validation.
    pub fn bypass_document_validation(mut self, enable: bool) -> Self {
        self.options.bypass_document_validation = Some(enable);
        self
    }

    /// If true, when an insert fails, return without performing the remaining writes. If false,
    /// when a write fails, continue with the remaining writes, if any.
    ///
    /// Defaults to true.
    pub fn ordered(mut self, enable: bool) -> Self {
        self.options.ordered = Some(enable);
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
    /// This method fails if:
    /// - any of the documents could not be converted into a BSON `Document`.
    /// - the mongodb encountered an error.
    pub async fn query(
        self,
        client: &Client,
        documents: Vec<C>,
    ) -> crate::Result<HashMap<usize, Bson>>
    where
        C: Collection,
    {
        let documents = documents
            .into_iter()
            .map(|s| s.into_document())
            .collect::<Result<Vec<Document>, _>>()?;
        client
            .database()
            .collection(C::COLLECTION)
            .insert_many(documents, self.options)
            .await
            .map(|r| r.inserted_ids)
            .map_err(crate::error::mongodb)
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
    /// - any of the documents could not be converted into a BSON `Document`.
    /// - the mongodb encountered an error.
    #[cfg(feature = "blocking")]
    pub fn blocking(
        self,
        client: &crate::blocking::Client,
        documents: Vec<C>,
    ) -> crate::Result<HashMap<usize, Bson>>
    where
        C: Collection,
    {
        let documents = documents
            .into_iter()
            .map(|s| s.into_document())
            .collect::<Result<Vec<Document>, _>>()
            .map_err(crate::error::bson)?;
        let resp = client.execute(crate::blocking::Request::Insert(
            C::COLLECTION,
            documents,
            self.options,
        ))?;
        if let crate::blocking::Response::Insert(r) = resp {
            return Ok(r.inserted_ids);
        }
        Err(crate::error::runtime(
            "incorrect response from blocking client",
        ))
    }
}
