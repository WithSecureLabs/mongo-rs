//! # Mongo
//!
//! The `mongod` crate aims to provide a convenient, higher-level wrapper on top of the official
//! `bson` & `mongodb` crates.
//!
//! It provides the following:
//!
//! - Async and [blocking][blocking] Clients
//! - [Bson][bson] extensions
//! - Strong typing through the use of traits
//!
//! The [`mongod::Client`][client] is asynchronous. For applications that need a synchronous
//! solution, the [`mongod::blocking`][blocking] API can be used.
//!
//! ## Making Collections
//!
//! Defining an example collection using derive.
//!
//! ```
//! # use mongod_derive::{Bson, Mongo};
//! #[derive(Bson, Mongo)]
//! #[mongo(collection="users", field, filter, update)]
//! pub struct User {
//!     name: String,
//!     age: Option<u32>,
//!     email: Option<String>,
//! }
//! ```
//!
//! ## Making requests
//!
//! This crate is opinionated, here are some examples on how to use it for interaction with
//! mongodb. For more complex interactions see the individual implementations:
//!
//! - [`Delete`](query::Delete): Delete documents from a collection
//! - [`Find`](query::Find): Fetch documents from a collection
//! - [`Insert`](query::Insert): Insert documents into a collection
//! - [`Replace`](query::Replace): Replace documents in a collection
//! - [`Update`](query::Update): Update documents in a collection
//!
//! ### Deleting
//!
//! Deleting a user from the users collection.
//!
//! ```no_run
//! # use mongod_derive::{Bson, Mongo};
//! # #[derive(Bson, Mongo)]
//! # #[mongo(collection="users", field, filter, update)]
//! # pub struct User {
//! #     name: String,
//! #     age: Option<u32>,
//! #     email: Option<String>,
//! # }
//! # async fn doc() -> Result<(), mongod::Error> {
//! use mongod::{AsFilter, Comparator};
//!
//! let client = mongod::Client::new();
//!
//! let mut filter = User::filter();
//! filter.name = Some(Comparator::Eq("foo".to_owned()));
//!
//! let deleted = client.delete::<User, _>(Some(filter)).await.unwrap();
//! println!("delete {} documents", deleted);
//! # Ok(())
//! # }
//! ```
//!
//! ### Fetching
//!
//! Fetching users from the users collection.
//!
//! ```no_run
//! # use std::convert::TryFrom;
//! # use mongod_derive::{Bson, Mongo};
//! use bson::Document;
//! # #[derive(Debug, Bson, Mongo)]
//! # #[mongo(collection="users", field, filter, update)]
//! # pub struct User {
//! #     name: String,
//! #     age: Option<u32>,
//! #     email: Option<String>,
//! # }
//! # async fn doc() -> Result<(), mongod::Error> {
//! use futures::stream::StreamExt;
//!
//! use mongod::Collection;
//!
//! let client = mongod::Client::new();
//!
//! let mut cursor = client.find::<User, _, Document>(None).await.unwrap();
//! while let Some(res) = cursor.next().await {
//!     if let Ok(doc) = res {
//!         let user: User = User::from_document(doc).unwrap();
//!         println!("{:?}", user);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//!T ### Inserting
//!
//! Inserting a user into the users collection.
//!
//! ```no_run
//! # use mongod_derive::{Bson, Mongo};
//! # #[derive(Debug, Bson, Mongo)]
//! # #[mongo(collection="users", field, filter, update)]
//! # pub struct User {
//! #     name: String,
//! #     age: Option<u32>,
//! #     email: Option<String>,
//! # }
//! # async fn doc() -> Result<(), mongod::Error> {
//! let client = mongod::Client::new();
//!
//! let user = User {
//!     name: "foo".to_owned(),
//!     age: None,
//!     email: None,
//! };
//!
//! let result = client.insert(vec![user]).await.unwrap();
//! println!("(index: oid) {:?}", result);
//! # Ok(())
//! # }
//! ```
//!
//! ### Replacing
//!
//! Replacing a user in the users collection.
//!
//! ```no_run
//! # use mongod_derive::{Bson, Mongo};
//! # #[derive(Debug, Bson, Mongo)]
//! # #[mongo(collection="users", field, filter, update)]
//! # pub struct User {
//! #     name: String,
//! #     age: Option<u32>,
//! #     email: Option<String>,
//! # }
//! # async fn doc() -> Result<(), mongod::Error> {
//! use mongod::{AsFilter, Comparator};
//!
//! let client = mongod::Client::new();
//!
//! let mut filter = User::filter();
//! filter.name = Some(Comparator::Eq("foo".to_owned()));
//!
//! let user = User {
//!     name: "foo".to_owned(),
//!     age: Some(0),
//!     email: None,
//! };
//!
//! let oid = client.replace_one(filter, user).await.unwrap();
//! println!("{:?}", oid);
//! # Ok(())
//! # }
//! ```
//!
//! ### Updating
//!
//! Updating a user in the users collection.
//!
//! ```no_run
//! # use mongod_derive::{Bson, Mongo};
//! # #[derive(Debug, Bson, Mongo)]
//! # #[mongo(collection="users", field, filter, update)]
//! # pub struct User {
//! #     name: String,
//! #     age: Option<u32>,
//! #     email: Option<String>,
//! # }
//! # async fn doc() -> Result<(), mongod::Error> {
//! use mongod::{AsFilter, Comparator, AsUpdate};
//!
//! let client = mongod::Client::new();
//!
//! let mut filter = User::filter();
//! filter.name = Some(Comparator::Eq("foo".to_owned()));
//!
//! let mut update = User::update();
//! update.age = Some(Some(0));
//!
//! let updates = mongod::Updates {
//!     set: Some(update),
//!     ..Default::default()
//! };
//!
//! let updated = client.update::<User, _, _>(filter, updates).await.unwrap();
//! println!("updated {} documents", updated);
//! # Ok(())
//! # }
//! ```
//!
//! ## Optional Features
//!
//! The following are a list of [Cargo Features][cargo-features] that cna be enabled or disabled:
//!
//! - **blocking**: Provides the [blocking][] client API.
//! - **chrono**: Provides the [chrono][chrono] support for the [`ext::bson`][ext-bson].
//! - **derive**: Provides the `derive` macros from the [mongo-derive][derive] crate.
//!
//! [blocking]: ./blocking/index.html
//! [bson]: https://docs.rs/bson
//! [client]: ./struct.Client.html
//! [chrono]: https://docs.rs/chrono
//! [derive]: ../mongod_derive/index.html
//! [ext-bson]: ./ext/bson/index.html
//! [schema]: ./schema/index.html
//! [cargo-features]: https://doc.rust-lang.org/stable/cargo/reference/manifest.html#the-features-section

#![deny(missing_docs)]
#![deny(unused_imports)]

#[macro_use]
pub extern crate bson;
#[allow(unused_imports)] // FIXME: Needed til we add logging
#[macro_use]
extern crate log;
pub extern crate mongodb as db;
#[macro_use]
extern crate serde;

pub use self::collection::Collection;
pub use self::error::{Error, Kind as ErrorKind};
pub use self::field::{AsField, Field};
pub use self::filter::{AsFilter, Comparator, Filter};
pub use self::query::Query;
pub use self::r#async::{Client, ClientBuilder};
pub use self::sort::{Order, Sort};
pub use self::update::{AsUpdate, Update, Updates};

pub(crate) use error::Result;

mod r#async;
#[cfg(feature = "blocking")]
pub mod blocking;
mod collection;
mod error;
pub mod ext;
mod field;
mod filter;
pub mod query;
mod sort;
mod update;

#[cfg(feature = "mongod-derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate mongod_derive;
#[cfg(feature = "mongod-derive")]
#[doc(hidden)]
pub use mongod_derive::*;
