use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use bson::oid::ObjectId;
use bson::Document;
use futures::StreamExt;
use mongodb::options::{
    Acknowledgment, AuthMechanism, Credential, ReadConcern, ReadConcernLevel, ReadPreference,
    ReadPreferenceOptions, SelectionCriteria, Tls, TlsOptions, WriteConcern,
};
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

        // NOTE: What we really want here is ClientOptionsParser, but its private... so lets try
        // and work around that with minimal code duplication
        let url = Url::parse(&uri).map_err(crate::error::builder)?;
        let mut options = mongodb::options::ClientOptions::builder()
            .hosts(vec![mongodb::options::ServerAddress::Tcp {
                host: url.host_str().unwrap_or("127.0.0.1").to_owned(),
                port: url.port(),
            }])
            .build();
        let mut kv = url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<String, String>>();
        if let Some(app_name) = kv.remove("appName") {
            options.app_name = Some(app_name);
        }
        if let Some(_compressors) = kv.remove("compressors") {
            // FIXME: Field is private...
            //options.compressors = Some(compressors.split(',').map(|v| v.to_string()).collect());
        }
        if let Some(connect_timeout) = kv.remove("connectTimeoutMS") {
            options.connect_timeout = Some(Duration::from_millis(
                str::parse(&connect_timeout).map_err(crate::error::builder)?,
            ));
        }
        if let Some(direct) = kv.remove("directConnection") {
            options.direct_connection = Some(str::parse(&direct).map_err(crate::error::builder)?);
        }
        if let Some(heartbeat_frequency) = kv.remove("heartbeatFrequencyMS") {
            options.heartbeat_freq = Some(Duration::from_millis(
                str::parse(&heartbeat_frequency).map_err(crate::error::builder)?,
            ));
        }
        if let Some(local_threshold) = kv.remove("localThresholdMS") {
            options.local_threshold = Some(Duration::from_millis(
                str::parse(&local_threshold).map_err(crate::error::builder)?,
            ));
        }
        if let Some(max_idle_time) = kv.remove("maxIdleTimeMS") {
            options.max_idle_time = Some(Duration::from_millis(
                str::parse(&max_idle_time).map_err(crate::error::builder)?,
            ));
        }
        if let Some(max_pool_size) = kv.remove("maxPoolSize") {
            options.max_pool_size =
                Some(str::parse(&max_pool_size).map_err(crate::error::builder)?);
        }
        if let Some(min_pool_size) = kv.remove("minPoolSize") {
            options.min_pool_size =
                Some(str::parse(&min_pool_size).map_err(crate::error::builder)?);
        }
        if let Some(read_concern_level) = kv.remove("readConcernLevel") {
            let level = match read_concern_level.as_str() {
                "local" => ReadConcernLevel::Local,
                "majority" => ReadConcernLevel::Majority,
                "linearizable" => ReadConcernLevel::Linearizable,
                "available" => ReadConcernLevel::Available,
                _ => ReadConcernLevel::Custom(read_concern_level),
            };
            options.read_concern = Some(ReadConcern::from(level));
        }
        if let Some(replica_set) = kv.remove("replicaSet") {
            options.repl_set_name = Some(replica_set);
        }
        if let Some(retry_reads) = kv.remove("retryReads") {
            options.retry_reads = Some(str::parse(&retry_reads).map_err(crate::error::builder)?);
        }
        if let Some(retry_writes) = kv.remove("retryWrites") {
            options.retry_writes = Some(str::parse(&retry_writes).map_err(crate::error::builder)?);
        }
        if let Some(server_selection_timeout) = kv.remove("serverSelectionTimeoutMS") {
            options.server_selection_timeout = Some(Duration::from_millis(
                str::parse(&server_selection_timeout).map_err(crate::error::builder)?,
            ));
        }
        if let Some(_socket_timeout) = kv.remove("socketTimeoutMS") {
            // FIXME: Field is private...
            //options.socket_timeout = Some(Duration::from_millis(
            //    str::parse(&socket_timeout).map_err(crate::error::builder)?,
            //));
        }
        if let Some(_uuid_representation) = kv.remove("uuidRepresentation") {
            // FIXME: Not supported by mongodb...
            //options.uuid_representation = Some(uuid_representation);
        }
        if let Some(_wait_queue_multiple) = kv.remove("waitQueueMultiple") {
            // FIXME: Not supported by mongodb...
            //options.wait_queue_multiple = Some(str::parse(&wait_queue_multiple).map_err(crate::error::builder)?);
        }
        if let Some(_wait_queue_timeout) = kv.remove("waitQueueTimeoutMS") {
            // FIXME: Not supported by mongodb...
            // options.wait_queue_timeout = Some(Duration::from_millis(
            //     str::parse(&wait_queue_timeout).map_err(crate::error::builder)?,
            // ));
        }
        if let Some(_zlib_compression_level) = kv.remove("zlibCompressionLevel") {
            // FIXME: Field is private...
            //options.zlib_compression_level = Some(str::parse(&zlib_compression_level).map_err(crate::error::builder)?);
        }

        let journal = kv.remove("journal");
        let w = kv.remove("w");
        let w_timeout = kv.remove("wTimeoutMS");
        if journal.is_some() || w.is_some() || w_timeout.is_some() {
            let mut write_concern = WriteConcern::default();
            if let Some(journal) = journal {
                write_concern.journal = Some(str::parse(&journal).map_err(crate::error::builder)?);
            }
            if let Some(w) = w {
                let w = match str::parse::<u32>(&w) {
                    Ok(n) => Acknowledgment::Nodes(n),
                    Err(_) => match w.as_str() {
                        "majority" => Acknowledgment::Majority,
                        _ => Acknowledgment::Custom(w),
                    },
                };
                write_concern.w = Some(w);
            }
            if let Some(w_timeout) = w_timeout {
                write_concern.w_timeout = Some(Duration::from_millis(
                    str::parse(&w_timeout).map_err(crate::error::builder)?,
                ));
            }
            options.write_concern = Some(write_concern);
        }

        let max_staleness = kv.remove("maxStalenessSeconds");
        let read_preference = kv.remove("readPreference");
        let read_preference_tags = kv.remove("readPreferenceTags");
        if max_staleness.is_some() || read_preference.is_some() || read_preference_tags.is_some() {
            let mut read_preference_options = ReadPreferenceOptions::default();
            if let Some(max_staleness) = max_staleness {
                read_preference_options.max_staleness = Some(Duration::from_secs(
                    str::parse(&max_staleness).map_err(crate::error::builder)?,
                ));
            }
            if let Some(read_preference_tags) = read_preference_tags {
                let mut tags: HashMap<String, String> = HashMap::new();
                for kv in read_preference_tags.split(',') {
                    let pair: Vec<String> = kv.split(':').map(|v| v.to_string()).collect();
                    if pair.len() != 2 {
                        return Err(crate::error::builder("tags must be kv pairs"));
                    }
                    tags.insert(pair[0].to_string(), pair[1].to_string());
                }
                read_preference_options.tag_sets = Some(vec![tags]);
            }
            if let Some(read_preference) = read_preference {
                let read = match read_preference.as_str() {
                    "primary" => ReadPreference::Primary,
                    "secondary" => ReadPreference::Secondary {
                        options: read_preference_options,
                    },
                    "primary_preferred" => ReadPreference::PrimaryPreferred {
                        options: read_preference_options,
                    },
                    "secondary_preferred" => ReadPreference::SecondaryPreferred {
                        options: read_preference_options,
                    },
                    "nearest" => ReadPreference::Nearest {
                        options: read_preference_options,
                    },
                    _ => {
                        return Err(crate::error::builder(format!(
                            "unknown read preference '{}'",
                            read_preference
                        )))
                    }
                };
                options.selection_criteria = Some(SelectionCriteria::ReadPreference(read));
            }
        }

        let auth_source = kv.remove("authSource");
        let auth_mechanism = kv.remove("authMechanism");
        let auth_mechanism_properties = kv.remove("authMechanismProperties");
        if url.username() != ""
            || self.username.is_some()
            || auth_source.is_some()
            || auth_mechanism.is_some()
            || auth_mechanism_properties.is_some()
        {
            let mut credentials = Credential::default();
            if self.username.is_some() || url.username() != "" {
                credentials.username = self.username.or(Some(url.username().to_string()));
                credentials.password = self.password.or(url.password().map(|p| p.to_string()));
            }
            if auth_source.is_some() {
                credentials.source = auth_source;
            }
            if let Some(auth_mechanism) = auth_mechanism {
                credentials.mechanism =
                    Some(AuthMechanism::from_str(&auth_mechanism).map_err(crate::error::builder)?);
            }
            if let Some(auth_mechanism_properties) = auth_mechanism_properties {
                let mut document = bson::Document::new();
                for kv in auth_mechanism_properties.split(',') {
                    let pair: Vec<String> = kv.split(':').map(|v| v.to_string()).collect();
                    if pair.len() != 2 {
                        return Err(crate::error::builder("properties must be kv pairs"));
                    }
                    document.insert(pair[0].to_string(), pair[1].to_string());
                }
                credentials.mechanism_properties = Some(document);
            }
            options.credential = Some(credentials);
        }

        let tls_enabled = kv.remove("tls");
        let tls_insecure = kv.remove("tlsInsecure");
        let tls_ca_file = kv.remove("tlsCAFile");
        let tls_certificate_key_file = kv.remove("tlsCertificateKeyFile");
        // FIXME: Not supported
        //let tls_certificate_key_file_password = kv.remove("tlsCertificateKeyFilePassword");
        let tls_allow_invalid_certificates = kv.remove("tlsAllowInvalidCertificates");
        // FIXME: Not supported
        //let tls_allow_invalid_hostnames = kv.remove("tlsAllowInvalidHostnames");
        if self.ca.is_some()
            || self.cert_key.is_some()
            || tls_enabled.is_some()
            || tls_insecure.is_some()
            || tls_ca_file.is_some()
            || tls_certificate_key_file.is_some()
            || tls_allow_invalid_certificates.is_some()
        {
            let enabled = match tls_enabled {
                Some(enabled) => enabled.parse::<bool>().map_err(crate::error::builder)?,
                None => true,
            };
            let tls = match enabled {
                true => {
                    let mut options = TlsOptions::default();
                    if let Some(tls_allow_invalid_certificates) = tls_allow_invalid_certificates {
                        options.allow_invalid_certificates = Some(
                            tls_allow_invalid_certificates
                                .parse()
                                .map_err(crate::error::builder)?,
                        );
                    } else if let Some(tls_insecure) = tls_insecure {
                        options.allow_invalid_certificates =
                            Some(tls_insecure.parse().map_err(crate::error::builder)?);
                    }
                    if let Some(ca_file_path) = self.ca.or(tls_ca_file) {
                        options.ca_file_path = Some(PathBuf::from(ca_file_path));
                    }
                    if let Some(cert_key_file_path) = self.cert_key.or(tls_certificate_key_file) {
                        options.cert_key_file_path = Some(PathBuf::from(cert_key_file_path));
                    }
                    Tls::Enabled(options)
                }
                false => Tls::Disabled,
            };
            options.tls = Some(tls);
        }

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
    pub fn collection<C>(&self) -> mongodb::Collection<Document>
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
    pub async fn delete<C, F>(&self, filter: Option<F>) -> crate::Result<u64>
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
    pub async fn find<C, F, T>(&self, filter: Option<F>) -> crate::Result<mongodb::Cursor<Document>>
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
