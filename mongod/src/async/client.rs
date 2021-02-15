use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use bson::oid::ObjectId;
use futures::StreamExt;
use mongodb::options::{Credential, Tls, TlsOptions};
use url::Url;

use crate::collection::Collection;
use crate::filter::{AsFilter, Filter};
use crate::query;
use crate::update::{AsUpdate, Update, Updates};

/// A `ClientBuilder` can be used to create a `Client` with custom configuration.
pub struct ClientBuilder {
    ca: Option<String>,
    cert_key: Option<String>,
    database: Option<String>,
    password: Option<String>,
    uri: Option<String>,
    username: Option<String>,
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
            ca: None,
            cert_key: None,
            database: None,
            password: None,
            uri: None,
            username: None,
        }
    }

    /// Returns a `Client` built from this `ClientBuilder` configuration.
    ///
    /// # Errors
    ///
    /// This method fails if the `mongodb::Client` cannot be initialised.
    pub fn build(self) -> crate::Result<Client> {
        let database = self.database.unwrap_or_else(|| String::from("db"));
        let uri = self
            .uri
            .unwrap_or_else(|| String::from("mongodb://127.0.0.1:27017"));

        // Init the client
        let mut credential = None;
        if self.username.is_some() {
            credential = Some(
                Credential::builder()
                    .username(self.username)
                    .password(self.password)
                    .build(),
            );
        }
        let mut tls = None;
        if self.ca.is_some() || self.cert_key.is_some() {
            tls = Some(Tls::Enabled(
                TlsOptions::builder()
                    .ca_file_path(self.ca)
                    .cert_key_file_path(self.cert_key)
                    .build(),
            ));
        }
        let url = Url::parse(&uri).map_err(crate::error::builder)?;
        let options = mongodb::options::ClientOptions::builder()
            .hosts(vec![mongodb::options::StreamAddress {
                hostname: url.host_str().unwrap_or("127.0.0.1").to_owned(),
                port: url.port(),
            }])
            .credential(credential)
            .tls(tls)
            .build();
        let client = mongodb::Client::with_options(options).map_err(crate::error::builder)?;

        Ok(Client {
            inner: Arc::new(ClientInner { client, database }),
        })
    }

    /// Sets the username/password that should be used by this client.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), mongod::Error> {
    ///     let _client = mongod::Client::builder()
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
        self.username = Some(username.to_string());
        if let Some(password) = password {
            self.password = Some(password.to_string());
        }
        self
    }

    /// Sets the CA file that should be used by this client for TLS.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), mongod::Error> {
    ///     let _client = mongod::Client::builder()
    ///         .ca("./certs/foo.pem")
    ///         .build().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn ca<I: Into<String>>(mut self, path: I) -> Self {
        self.ca = Some(path.into());
        self
    }

    /// Sets the certificate file that should be used by this client for identification.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), mongod::Error> {
    ///     let _client = mongod::Client::builder()
    ///         .cert_key("./certs/foo.pem")
    ///         .build().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn cert_key<I: Into<String>>(mut self, path: I) -> Self {
        self.cert_key = Some(path.into());
        self
    }

    /// Sets the database that should be used by this client.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), mongod::Error> {
    ///     let _client = mongod::Client::builder()
    ///         .database("foo")
    ///         .build().unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub fn database<I: Into<String>>(mut self, database: I) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Sets the uri that this client should use to connect to a mongo instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), mongod::Error> {
    ///     let _client = mongod::Client::builder()
    ///         .uri("mongodb://foo")
    ///         .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn uri<I: Into<String>>(mut self, uri: I) -> Self {
        self.uri = Some(uri.into());
        self
    }
}

/// An asynchronous `Client` to query mongo with.
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

struct ClientInner {
    client: mongodb::Client,
    database: String,
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
    pub fn from_client<I: Into<String>>(client: mongodb::Client, database: I) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                client,
                database: database.into(),
            }),
        }
    }

    /// Returns the `mongodb::Client`
    pub fn client(&self) -> mongodb::Client {
        self.inner.client.to_owned()
    }

    /// Returns the `mongodb::Document` from the mongodb.
    pub fn collection<C>(&self) -> mongodb::Collection
    where
        C: Collection,
    {
        self.inner
            .client
            .database(&self.inner.database)
            .collection(C::COLLECTION)
    }

    /// Returns the `mongodb::Database` from the mongodb.
    pub fn database(&self) -> mongodb::Database {
        self.inner.client.database(&self.inner.database)
    }

    /// Convenience method to delete documents from a collection using a given filter.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub async fn delete<C, F>(&self, filter: Option<F>) -> crate::Result<i64>
    where
        C: AsFilter<F> + Collection,
        F: Filter,
    {
        let mut delete = query::Delete::<C>::new().many(true);
        if let Some(filter) = filter {
            delete = delete.filter::<F>(filter)?
        }
        delete.query(&self).await
    }

    /// Convenience method to delete one document from a collection using a given filter.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub async fn delete_one<C, F>(&self, filter: F) -> crate::Result<bool>
    where
        C: AsFilter<F> + Collection,
        F: Filter,
    {
        let deleted = query::Delete::<C>::new()
            .many(false)
            .filter::<F>(filter)?
            .query(&self)
            .await?;
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
    pub async fn find<C, F>(&self, filter: Option<F>) -> crate::Result<mongodb::Cursor>
    where
        C: AsFilter<F> + Collection,
        F: Filter,
    {
        let mut find: query::Find<C> = query::Find::new();
        if let Some(filter) = filter {
            find = find.filter(filter)?;
        }
        find.query(&self).await
    }

    /// Convenience method to find a document in a collection using a given filter.
    ///
    /// This function is mainly intended for use cases where the filter is known to return unique
    /// hits. If you need something more complicated use `find` or the `FindBuilder`.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error, or if the found document is invalid.
    pub async fn find_one<C, F>(&self, filter: F) -> crate::Result<Option<C>>
    where
        C: AsFilter<F> + Collection,
        F: Filter,
    {
        // NOTE: We don't wanna make another builder so we just eat the cost of getting a cursor...
        let find: query::Find<C> = query::Find::new();
        let mut cursor = find.filter(filter)?.query(&self).await?;
        if let Some(res) = cursor.next().await {
            let doc = res.map_err(crate::error::mongodb)?;
            let document: C = C::from_document(doc).map_err(crate::error::bson)?;
            return Ok(Some(document));
        }
        Ok(None)
    }

    /// Convenience method to insert documents in a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error, or if the found document is invalid.
    pub async fn insert<C>(&self, documents: Vec<C>) -> crate::Result<HashMap<usize, ObjectId>>
    where
        C: Collection,
    {
        let result = query::Insert::new().query(&self, documents).await?;
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
    pub async fn insert_one<C>(&self, document: C) -> crate::Result<ObjectId>
    where
        C: Collection,
    {
        // NOTE: We don't wanna make another builder so we just eat the cost of allocating a vec...
        let result = query::Insert::new().query(&self, vec![document]).await?;
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
    pub async fn replace_one<C, F>(&self, filter: F, document: C) -> crate::Result<bool>
    where
        C: AsFilter<F> + Collection,
        F: Filter,
    {
        query::Replace::new()
            .filter::<F>(filter)?
            .query(&self, document)
            .await
    }

    /// Convenience method to update documents in a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub async fn update<C, F, U>(&self, filter: F, updates: Updates<U>) -> crate::Result<i64>
    where
        C: AsFilter<F> + AsUpdate<U> + Collection,
        F: Filter,
        U: Update,
    {
        let updated = query::Update::<C>::new()
            .filter::<F>(filter)?
            .query::<U>(&self, updates)
            .await?;
        Ok(updated)
    }

    /// Convenience method to update one document from a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub async fn update_one<C, F, U>(&self, filter: F, updates: Updates<U>) -> crate::Result<bool>
    where
        C: AsFilter<F> + AsUpdate<U> + Collection,
        F: Filter,
        U: Update,
    {
        let updated = query::Update::<C>::new()
            .many(false)
            .filter::<F>(filter)?
            .query::<U>(&self, updates)
            .await?;
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
    pub async fn upsert<C, F, U>(&self, filter: F, updates: Updates<U>) -> crate::Result<i64>
    where
        C: AsFilter<F> + AsUpdate<U> + Collection,
        F: Filter,
        U: Update,
    {
        let updated = query::Update::<C>::new()
            .upsert(true)
            .filter::<F>(filter)?
            .query::<U>(&self, updates)
            .await?;
        Ok(updated)
    }

    /// Convenience method to upsert one document from a collection.
    ///
    /// # Errors
    ///
    /// This method fails if the mongodb encountered an error.
    pub async fn upsert_one<C, F, U>(&self, filter: F, updates: Updates<U>) -> crate::Result<bool>
    where
        C: AsFilter<F> + AsUpdate<U> + Collection,
        F: Filter,
        U: Update,
    {
        let updated = query::Update::<C>::new()
            .many(false)
            .upsert(true)
            .filter::<F>(filter)?
            .query::<U>(&self, updates)
            .await?;
        if updated > 0 {
            return Ok(true);
        }
        Ok(false)
    }
}
