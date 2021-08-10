use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::thread;

use bson::oid::ObjectId;
use bson::Document;
use mongodb::options::{
    DeleteOptions, FindOptions, InsertManyOptions, ReplaceOptions, UpdateOptions,
};
use mongodb::results::{DeleteResult, InsertManyResult, UpdateResult};

use super::cursor::{Cursor, TypedCursor};
use crate::collection::Collection;
use crate::filter::{AsFilter, Filter};
use crate::query;
use crate::r#async;
use crate::update::{AsUpdate, Update, Updates};

/// A `ClientBuilder` can be used to create a `Client` with custom configuration.
pub struct ClientBuilder {
    builder: r#async::ClientBuilder,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientBuilder {
    /// Constructs a new `ClientBuilder`.
    ///
    /// This is the same as `Client::Builder()`.
    pub fn new() -> Self {
        Self {
            builder: r#async::ClientBuilder::new(),
        }
    }

    /// Returns a `Client` built from this `ClientBuilder` configuration.
    ///
    /// # Errors
    ///
    /// This method fails if the `mongodb::Client` cannot be initialised.
    pub fn build(self) -> crate::Result<Client> {
        Ok(Client {
            inner: Arc::new(ClientInner::new(self, None)?),
        })
    }

    /// Sets the username/password that should be used by this client.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), mongod::Error> {
    ///     let _client = mongod::blocking::Client::builder()
    ///         .auth("foo", Some("bar"))
    ///         .build().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn auth<U, P>(mut self, username: U, password: Option<P>) -> Self
    where
        U: Display,
        P: Display,
    {
        self.builder = self.builder.auth(username, password);
        self
    }

    /// Sets the CA file that should be used by this client for TLS.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), mongod::Error> {
    ///     let _client = mongod::blocking::Client::builder()
    ///         .ca("./certs/foo.pem")
    ///         .build().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn ca<I: Into<String>>(mut self, path: I) -> Self {
        self.builder = self.builder.ca(path);
        self
    }

    /// Sets the certificate file that should be used by this client for identification.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), mongod::Error> {
    ///     let _client = mongod::blocking::Client::builder()
    ///         .cert_key("./certs/foo.pem")
    ///         .build().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn cert_key<I: Into<String>>(mut self, path: I) -> Self {
        self.builder = self.builder.cert_key(path);
        self
    }

    /// Sets the database that should be used by this client.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), mongod::Error> {
    ///     let _client = mongod::blocking::Client::builder()
    ///         .database("foo")
    ///         .build().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn database<I: Into<String>>(mut self, database: I) -> Self {
        self.builder = self.builder.database(database);
        self
    }

    /// Sets the uri that this client should use to connect to a mongo instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), mongod::Error> {
    ///     let _client = mongod::blocking::Client::builder()
    ///         .uri("mongodb://foo")
    ///         .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn uri<I: Into<String>>(mut self, uri: I) -> Self {
        self.builder = self.builder.uri(uri);
        self
    }
}

/// A synchronous `Client` to query mongo with.
///
/// The client uses sane defaults but these can be tweaked using the builder. To configure a
/// `Client`, use `Client::builder`.
///
/// The `Client` holds a connection pool internally, so it is advised that you create once, and
/// reuse it.
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum Request {
    Delete(bool, &'static str, Document, DeleteOptions),
    Find(&'static str, Option<Document>, FindOptions),
    Insert(&'static str, Vec<Document>, InsertManyOptions),
    Replace(&'static str, Document, Document, ReplaceOptions),
    Update(bool, &'static str, Document, Document, UpdateOptions),
}
pub(crate) enum Response {
    Delete(DeleteResult),
    Find(Cursor),
    Insert(InsertManyResult),
    Replace(UpdateResult),
    Update(UpdateResult),
}
type OneshotResponse = std::sync::mpsc::Sender<crate::Result<Response>>;
type ThreadSender = tokio::sync::mpsc::UnboundedSender<(Request, OneshotResponse)>;

struct ClientInner {
    _thread: Option<thread::JoinHandle<()>>,
    tx: ThreadSender,
}

impl Client {
    /// Constructs a new `Client`.
    ///
    /// # Panics
    ///
    /// This method panics if the `mongodb::Client` fails to initialise.
    ///
    /// Use `Client::builder()` if you wish to handle this failure as an `Error` instead of
    /// panicking.
    pub fn new() -> Self {
        ClientBuilder::new().build().expect("Client::new()")
    }

    /// Creates a `ClientInner` to configure a `Client`.
    ///
    /// This is the same as `ClientBuilder::new()`.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Constructs a new `Client` using a `mongodb::Client`.
    pub fn from_client<I: Into<String>>(
        client: mongodb::Client,
        database: I,
    ) -> crate::Result<Self> {
        Ok(Self {
            inner: Arc::new(ClientInner::new(
                ClientBuilder::new(),
                Some(crate::r#async::Client::from_client(client, database)),
            )?),
        })
    }

    /// Convenience method to delete documents from a collection using a given filter.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub fn delete<C, F>(&self, filter: Option<F>) -> crate::Result<u64>
    where
        C: AsFilter<F> + Collection,
        F: Filter,
    {
        let mut delete = query::Delete::<C>::new().many(true);
        if let Some(filter) = filter {
            delete = delete.filter(filter)?;
        }
        delete.blocking(&self)
    }

    /// Convenience method to delete one document from a collection using a given filter.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub fn delete_one<C, F>(&self, filter: F) -> crate::Result<bool>
    where
        C: AsFilter<F> + Collection,
        F: Filter,
    {
        let deleted = query::Delete::<C>::new()
            .many(false)
            .filter::<F>(filter)?
            .blocking(&self)?;
        Ok(deleted > 0)
    }

    /// Convenience method to find documents in a collection.
    ///
    /// This function is mainly intended for use cases where the filter is known to return unique
    /// hits. If you need something more complicated use `find` or the `FindBuilder`.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub fn find<C, F>(&self, filter: Option<F>) -> crate::Result<TypedCursor<C>>
    where
        C: AsFilter<F> + Collection,
        F: Filter,
    {
        let mut find: query::Find<C> = query::Find::new();
        if let Some(filter) = filter {
            find = find.filter(filter)?;
        }
        find.blocking(&self)
    }

    /// Convenience method to find a document in a collection using a given filter.
    ///
    /// This function is mainly intended for use cases where the filter is known to return unique
    /// hits. If you need something more complicated use `find` or the `FindBuilder`.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error, or if the found document is invalid.
    pub fn find_one<C, F>(&self, filter: F) -> crate::Result<Option<(ObjectId, C)>>
    where
        C: AsFilter<F> + Collection,
        F: Filter,
    {
        // NOTE: We don't wanna make another builder so we just eat the cost of getting a cursor...
        let find: query::Find<C> = query::Find::new();
        let mut cursor = find.filter(filter)?.blocking(&self)?;
        if let Some(res) = cursor.next() {
            return Ok(Some(res?));
        }
        Ok(None)
    }

    /// Convenience method to insert documents in a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error, or if the found document is invalid.
    pub fn insert<C>(&self, documents: Vec<C>) -> crate::Result<HashMap<usize, ObjectId>>
    where
        C: Collection,
    {
        let result = query::Insert::new().blocking(&self, documents)?;
        Ok(result
            .into_iter()
            .filter_map(|(k, v)| match v {
                bson::Bson::ObjectId(id) => Some((k, id)),
                _ => None,
            })
            .collect())
    }

    /// Convenience method to insert a document in a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error, or if the found document is invalid.
    pub fn insert_one<C>(&self, document: C) -> crate::Result<ObjectId>
    where
        C: Collection,
    {
        // NOTE: We don't wanna make another builder so we just eat the cost of allocating a vec...
        let result = query::Insert::new().blocking(&self, vec![document])?;
        if let Some((_, v)) = result.into_iter().next() {
            if let bson::Bson::ObjectId(id) = v {
                return Ok(id);
            }
        }
        Err(crate::error::mongodb(
            "failed to insert document into mongo",
        ))
    }

    /// Convenience method to replace a document in a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub fn replace_one<C, F>(&self, filter: F, document: C) -> crate::Result<bool>
    where
        C: AsFilter<F> + Collection,
        F: Filter,
    {
        query::Replace::new()
            .filter::<F>(filter)?
            .blocking(&self, document)
    }

    /// Convenience method to update documents in a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub fn update<C, F, U>(&self, filter: F, updates: Updates<U>) -> crate::Result<u64>
    where
        C: AsFilter<F> + AsUpdate<U> + Collection,
        F: Filter,
        U: Update,
    {
        let updated = query::Update::<C>::new()
            .filter::<F>(filter)?
            .blocking::<U>(&self, updates)?;
        Ok(updated)
    }

    /// Convenience method to update one document from a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub fn update_one<C, F, U>(&self, filter: F, updates: Updates<U>) -> crate::Result<bool>
    where
        C: AsFilter<F> + AsUpdate<U> + Collection,
        F: Filter,
        U: Update,
    {
        let updated = query::Update::<C>::new()
            .many(false)
            .filter::<F>(filter)?
            .blocking::<U>(&self, updates)?;
        if updated > 0 {
            return Ok(true);
        }
        Ok(false)
    }

    /// Convenience method to upsert documents from a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub fn upsert<C, F, U>(&self, filter: F, updates: Updates<U>) -> crate::Result<u64>
    where
        C: AsFilter<F> + AsUpdate<U> + Collection,
        F: Filter,
        U: Update,
    {
        let updated = query::Update::<C>::new()
            .upsert(true)
            .filter::<F>(filter)?
            .blocking::<U>(&self, updates)?;
        Ok(updated)
    }

    /// Convenience method to upsert one document from a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub fn upsert_one<C, F, U>(&self, filter: F, updates: Updates<U>) -> crate::Result<bool>
    where
        C: AsFilter<F> + AsUpdate<U> + Collection,
        F: Filter,
        U: Update,
    {
        let updated = query::Update::<C>::new()
            .many(false)
            .upsert(true)
            .filter::<F>(filter)?
            .blocking::<U>(&self, updates)?;
        if updated > 0 {
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn execute(&self, req: Request) -> crate::Result<Response> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.inner
            .tx
            .send((req, tx))
            .map_err(|_| crate::error::runtime("failed to send request to blocking thread"))?;
        rx.recv().map_err(crate::error::runtime)?
    }
}

impl ClientInner {
    fn new(builder: ClientBuilder, client: Option<crate::r#async::Client>) -> crate::Result<Self> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(Request, OneshotResponse)>();
        let (spawn_tx, spawn_rx) = std::sync::mpsc::channel::<crate::Result<()>>();
        let handle = thread::Builder::new()
            .name("mongo-blocking-runtime".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(crate::error::builder)
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        if let Err(e) = spawn_tx.send(Err(e)) {
                            error!("failed to communicate runtime builder: {:?}", e);
                        }
                        return;
                    }
                };
                let f = async move {
                    let client = match client {
                        Some(client) => client,
                        None => match builder.builder.build().map_err(crate::error::builder) {
                            Ok(client) => client,
                            Err(e) => {
                                if let Err(e) = spawn_tx.send(Err(e)) {
                                    error!("failed to create async client: {:?}", e);
                                }
                                return;
                            }
                        },
                    };
                    if let Err(e) = spawn_tx.send(Ok(())) {
                        error!("failed to communicate successful startup: {:?}", e);
                        return;
                    }
                    let database = client.database();
                    while let Some((req, req_tx)) = rx.recv().await {
                        let resp = match req {
                            Request::Delete(many, collection, filter, options) => if many {
                                database
                                    .collection::<Document>(collection)
                                    .delete_many(filter, options)
                                    .await
                            } else {
                                database
                                    .collection::<Document>(collection)
                                    .delete_one(filter, options)
                                    .await
                            }
                            .map(Response::Delete)
                            .map_err(crate::error::mongodb),
                            Request::Find(collection, filter, options) => {
                                match database.collection(collection).find(filter, options).await {
                                    Ok(c) => Ok(Response::Find(Cursor::new(c))),
                                    Err(e) => Err(crate::error::mongodb(e)),
                                }
                            }
                            Request::Insert(collection, documents, options) => database
                                .collection(collection)
                                .insert_many(documents, options)
                                .await
                                .map(Response::Insert)
                                .map_err(crate::error::mongodb),
                            Request::Replace(collection, filter, documents, options) => database
                                .collection(collection)
                                .replace_one(filter, documents, options)
                                .await
                                .map(Response::Replace)
                                .map_err(crate::error::mongodb),
                            Request::Update(many, collection, filter, updates, options) => {
                                if many {
                                    database
                                        .collection::<Document>(collection)
                                        .update_many(filter, updates, options)
                                        .await
                                } else {
                                    database
                                        .collection::<Document>(collection)
                                        .update_one(filter, updates, options)
                                        .await
                                }
                                .map(Response::Update)
                                .map_err(crate::error::mongodb)
                            }
                        };
                        let _ = req_tx.send(resp);
                    }
                };
                rt.block_on(f);
            })
            .map_err(crate::error::builder)?;

        if let Err(e) = spawn_rx.recv().map_err(crate::error::builder)? {
            return Err(e);
        }

        Ok(Self {
            _thread: Some(handle),
            tx,
        })
    }
}
