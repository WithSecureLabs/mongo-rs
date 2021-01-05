use std::marker::PhantomData;
use std::time::Duration;

use bson::Document;
use mongodb::options::{Collation, CursorType, FindOptions, Hint, ReadConcern, SelectionCriteria};
use mongodb::Cursor;

use crate::collection::Collection;
use crate::field::{AsField, Field};
use crate::filter::{AsFilter, Filter};
use crate::r#async::Client;
use crate::sort::Sort;

/// A querier to find documents in a MongoDB collection.
///
/// # Examples
///
/// Find all documents from a collection.
///
/// ```no_run
/// # async fn doc() -> Result<(), mongod::Error> {
/// # use mongod_derive::{Bson, Mongo};
///
/// use futures::stream::StreamExt;
///
/// use mongod::Collection;
///
/// #[derive(Debug, Bson, Mongo)]
/// #[mongo(collection="users", field, filter, update)]
/// pub struct User {
///     name: String,
/// }
///
/// let client = mongod::Client::new();
///
/// let mut cursor = mongod::query::Find::<User>::new()
///     .query(&client)
///     .await
///     .unwrap();
/// while let Some(res) = cursor.next().await {
///     if let Ok(doc) = res {
///         let user: User = User::from_document(doc).unwrap();
///         println!("{:?}", user);
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Find<C: Collection> {
    filter: Option<Document>,
    options: FindOptions,

    query_type: PhantomData<C>,
}

impl<C: Collection> Default for Find<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Collection> Find<C> {
    /// Constructs a `Find` querier.
    pub fn new() -> Self {
        Self {
            filter: None,
            options: FindOptions::default(),

            query_type: PhantomData,
        }
    }

    /// Enables writing to temporary files by the server.
    ///
    /// When set to true, the find operation can write data to the _tmp subdirectory in the dbPath
    /// directory.
    pub fn allow_disk_use(mut self, enable: bool) -> Self {
        self.options.allow_partial_results = Some(enable);
        self
    }

    /// Enables partial results.
    ///
    /// If true, partial results will be returned from a mongodb rather than an error being returned
    /// if one or more shards is down.
    pub fn allow_partial_results(mut self, enable: bool) -> Self {
        self.options.allow_partial_results = Some(enable);
        self
    }

    /// The number of documents the server should return per cursor batch.
    ///
    /// # Notes
    ///
    /// This does not have any affect on the documents that are returned by a cursor, only the
    /// number of documents kept in memory at a given time (and by extension, the number of round
    /// trips needed to return the entire set of documents returned by the query.
    pub fn batch_size(mut self, size: u32) -> Self {
        self.options.batch_size = Some(size);
        self
    }

    /// The collation to use for the operation.
    ///
    /// Collation allows users to specify language-specific rules for string comparison, such as
    /// rules for lettercase and accent marks.
    pub fn collation(mut self, value: Collation) -> Self {
        self.options.collation = Some(value);
        self
    }

    /// Tags the query with an arbitrary string.
    ///
    /// Used to help trace the operation through the database profiler, currentOp and logs.
    pub fn comment(mut self, value: String) -> Self {
        self.options.comment = Some(value);
        self
    }

    /// The type of cursor to return.
    pub fn cursor_type(mut self, r#type: CursorType) -> Self {
        self.options.cursor_type = Some(r#type);
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

    /// The maximum number of documents to query.
    ///
    /// If a negative number is specified, the documents will be returned in a single batch limited
    /// in number by the positive value of the specified limit.
    pub fn limit(mut self, value: i64) -> Self {
        self.options.limit = Some(value);
        self
    }

    /// The exclusive upper bound for a specific index.
    pub fn max(mut self, document: Document) -> Self {
        self.options.max = Some(document);
        self
    }

    /// The maximum amount of time for the server to wait on new documents to satisfy a tailable cursor query.
    ///
    /// If the cursor is not tailable, this option is ignored.
    pub fn max_await_time(mut self, duration: Duration) -> Self {
        self.options.max_await_time = Some(duration);
        self
    }

    /// Maximum number of documents or index keys to scan when executing the query.
    ///
    /// Notes
    ///
    /// This option is deprecated starting in MongoDB version 4.0 and removed in MongoDB 4.2. Use
    /// the max_time option instead.
    pub fn max_scan(mut self, value: i64) -> Self {
        self.options.max_scan = Some(value);
        self
    }

    /// The maximum amount of time to allow the query to run.
    ///
    /// This options maps to the maxTimeMS MongoDB query option, so the duration will be sent
    /// across the wire as an integer number of milliseconds.
    pub fn max_time(mut self, duration: Duration) -> Self {
        self.options.max_time = Some(duration);
        self
    }

    /// The inclusive lower bound for a specific index.
    pub fn min(mut self, document: Document) -> Self {
        self.options.min = Some(document);
        self
    }

    /// Whether the server should close the cursor after a period of inactivity.
    pub fn no_cursor_timeout(mut self, enable: bool) -> Self {
        self.options.no_cursor_timeout = Some(enable);
        self
    }

    /// Limits the fields of the document being returned.
    pub fn projection(mut self, document: Document) -> Self {
        self.options.projection = Some(document);
        self
    }

    /// The read concern to use for this find query.
    ///
    /// If none specified, the default set on the collection will be used.
    pub fn read_concern(mut self, concern: ReadConcern) -> Self {
        self.options.read_concern = Some(concern);
        self
    }

    /// Whether to return only the index keys in the documents.
    pub fn return_key(mut self, enable: bool) -> Self {
        self.options.return_key = Some(enable);
        self
    }

    /// The criteria used to select a server for this find query.
    ///
    /// If none specified, the default set on the collection will be used.
    pub fn selection_criteria(mut self, criteria: SelectionCriteria) -> Self {
        self.options.selection_criteria = Some(criteria);
        self
    }

    /// Whether to return the record identifier for each document.
    pub fn show_record_id(mut self, enable: bool) -> Self {
        self.options.show_record_id = Some(enable);
        self
    }

    /// The number of documents to skip before counting.
    pub fn skip(mut self, value: i64) -> Self {
        self.options.skip = Some(value);
        self
    }

    /// The order in which to sort the documents of the operation.
    pub fn sort<F>(mut self, sort: Sort<F>) -> Self
    where
        C: AsField<F>,
        F: Field + Into<String>,
    {
        self.options.sort = Some(sort.into_document());
        self
    }

    /// Query the database with this querier.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub async fn query(self, client: &Client) -> crate::Result<Cursor> {
        client
            .database()
            .collection(C::COLLECTION)
            .find(self.filter, self.options)
            .await
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
    /// This method fails if the mongodb encountered an error.
    #[cfg(feature = "blocking")]
    pub fn blocking(
        self,
        client: &crate::blocking::Client,
    ) -> crate::Result<crate::blocking::Cursor> {
        let resp = client.execute(crate::blocking::Request::Find(
            C::COLLECTION,
            self.filter,
            self.options,
        ))?;
        if let crate::blocking::Response::Find(r) = resp {
            return Ok(r);
        }
        Err(crate::error::runtime(
            "incorrect response from blocking client",
        ))
    }
}
