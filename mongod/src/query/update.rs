use std::marker::PhantomData;

use bson::Document;
use mongodb::options::{Collation, Hint, UpdateOptions, WriteConcern};

use crate::collection::Collection;
use crate::filter::{AsFilter, Filter};
use crate::r#async::Client;
use crate::update::{AsUpdate, Updates};

/// A querier to update documents in a MongoDB collection.
///
/// # Examples
///
/// Updates some documents in a collection.
///
/// ```no_run
/// # use mongod::{Bson, Mongo};
/// #[derive(Bson, Mongo)]
/// #[mongo(collection="users", field, filter, update)]
/// pub struct User {
///     name: String,
///     age: Option<u32>,
///     email: Option<String>,
/// }
/// # async fn doc() -> Result<(), mongod::Error> {
/// use mongod::{AsFilter, Comparator, AsUpdate};
///
/// let client = mongod::Client::new();
///
/// let mut filter = User::filter();
/// filter.name = Some(Comparator::Eq("foo".to_owned()));
///
/// let mut update = User::update();
/// update.name = Some("bar".to_owned());
///
/// let updates = mongod::Updates {
///     set: Some(update),
///     ..Default::default()
/// };
///
/// let updated = mongod::query::Update::<User>::new()
///     .filter(filter)
///     .unwrap()
///     .query(&client, updates)
///     .await
///     .unwrap();
///
/// println!("updated {} documents", updated);
/// # Ok(())
/// # }
/// ```
///
#[derive(Clone)]
pub struct Update<C: Collection> {
    filter: Option<Document>,
    many: bool,
    options: UpdateOptions,

    query_type: std::marker::PhantomData<C>,
}

impl<C: Collection> Default for Update<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Collection> Update<C> {
    /// Constructs an `Update` querier.
    pub fn new() -> Self {
        Self {
            filter: None,
            many: true,
            options: UpdateOptions::default(),

            query_type: PhantomData,
        }
    }

    /// An array of filters specifying to which array elements an update should apply.
    pub fn array_filters(mut self, filters: Vec<Document>) -> Self {
        self.options.array_filters = Some(filters);
        self
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

    /// Enable update many for this operation.
    ///
    /// Removes all documents that match the filter from a collection.
    pub fn many(mut self, enable: bool) -> Self {
        self.many = enable;
        self
    }

    /// Insert a document if no matching document is found.
    pub fn upsert(mut self, enable: bool) -> Self {
        self.options.upsert = Some(enable);
        self
    }

    ///The write concern for the operation.
    pub fn write_concern(mut self, concern: WriteConcern) -> Self {
        self.options.write_concern = Some(concern);
        self
    }

    /// Query the database with this querier.
    ///
    /// # Errors
    ///
    /// This method fails if:
    /// - the updates could not be converted into a BSON `Document`.
    /// - the mongodb encountered an error.
    pub async fn query<U>(self, client: &Client, updates: Updates<U>) -> crate::Result<i64>
    where
        C: AsUpdate<U>,
        U: crate::update::Update,
    {
        let filter = match self.filter {
            Some(f) => f,
            None => bson::Document::new(),
        };
        let result = if self.many {
            client
                .database()
                .collection(C::COLLECTION)
                .update_many(filter, updates.into_document()?, self.options)
                .await
        } else {
            client
                .database()
                .collection(C::COLLECTION)
                .update_one(filter, updates.into_document()?, self.options)
                .await
        }
        .map_err(crate::error::mongodb)?;
        Ok(result.matched_count)
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
    /// - the updates could not be converted into a BSON `Document`.
    /// - the mongodb encountered an error.
    #[cfg(feature = "blocking")]
    pub fn blocking<U>(
        self,
        client: &crate::blocking::Client,
        updates: Updates<U>,
    ) -> crate::Result<i64>
    where
        C: AsUpdate<U>,
        U: crate::update::Update,
    {
        let filter = match self.filter {
            Some(f) => f,
            None => bson::Document::new(),
        };
        let resp = client.execute(crate::blocking::Request::Update(
            self.many,
            C::COLLECTION,
            filter,
            updates.into_document()?,
            self.options,
        ))?;
        if let crate::blocking::Response::Update(r) = resp {
            return Ok(r.matched_count);
        }
        Err(crate::error::runtime(
            "incorrect response from blocking client",
        ))
    }
}
