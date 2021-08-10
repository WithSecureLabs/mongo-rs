//! A blocking Client API.
//!
//! The blocking `Client` will block the current thread to execute, instead of returning futures
//! that need to be executed on a runtime.
//!
//! # Optional
//!
//! This requires the optional `blocking` feature to be enabled.
//!
//! # Making requests
//!
//! This client functions in the same way as the async `Client` except it blocks, here is an
//! example to fetch users from a collection.
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
//! use mongod::Collection;
//!
//! let client = mongod::blocking::Client::new();
//!
//! let mut cursor = client.find::<User, _>(None).unwrap();
//! while let Some(res) = cursor.next() {
//!     if let Ok(user) = res {
//!         println!("{:?}", user);
//!     }
//! }
//! ```

mod client;
mod cursor;

pub use self::client::{Client, ClientBuilder};
pub(crate) use self::client::{Request, Response};
pub use self::cursor::{Cursor, TypedCursor};
