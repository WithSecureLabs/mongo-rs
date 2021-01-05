//! # Mongo Derive
//!
//! This crate provides two derives `Bson` & `Mongo` for the [`mongo`][mongo] crate.
//!
//! ## Examples
//!
//! Deriving BSON
//!
//! ```
//! # use mongo_derive::Bson;
//! #[derive(Bson)]
//! pub struct User {
//!     name: String,
//!     age: u32,
//! }
//! ```
//!
//! Deriving Mongo
//!
//! ```
//! # mod wrap {
//! # use mongo_derive::Mongo;
//! # #[derive(mongo_derive::Bson)]
//! #[derive(Mongo)]
//! #[mongo(collection = "users", field, filter, update)]
//! pub struct User {
//!     name: String,
//!     age: u32,
//! }
//! # }
//! ```
//!
//! [mongo]: https://docs.rs/mongo
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;
use syn::DeriveInput;

mod ast;
mod bson;
mod mongo;

/// Derives implementations for `TryFrom` so that the decorated type can be converted `to` & `from`
/// BSON.
///
/// ## Container Attributes
///
/// - #[bson(from)]: derives `TryFrom` on `Bson` for `type`
/// - #[bson(into)]: derives `TryFrom` on `type` for `Bson`
///
/// ### `#[bson(from)]`
///
/// Tells the derive to only implement the `from` parts of the derive, i.e. deserialising only.
///
/// ```
/// # use mongo_derive::Bson;
/// use std::convert::TryFrom;
///
/// #[derive(Debug, Bson)]
/// #[bson(from)]
/// struct User {
///     name: String,
/// }
/// let mut doc = mongo::bson::Document::new();
/// doc.insert("name", "foo".to_owned());
/// let bson = mongo::bson::Bson::Document(doc);
///
/// let user = User::try_from(bson).unwrap();
///
/// println!("{:?}", user);
/// ```
///
/// ### `#[bson(into)]`
///
/// Tells the derive to only implement the `into` parts of the derive, i.e. serialising only.
///
/// ```
/// # use mongo_derive::Bson;
/// use std::convert::TryFrom;
///
/// #[derive(Bson)]
/// #[bson(into)]
/// struct User {
///     name: String,
/// }
///
/// let user = User { name: "foo".to_owned() };
///
/// let bson = mongo::bson::Bson::try_from(user).unwrap();
///
/// println!("{:?}", bson);
/// ```
///
/// ## Field Attributes
///
/// - #[bson(serde)]
///
/// ### `#[bson(serde)]`
///
/// Tells the derive to use `serde` for the decorated field.
/// ```
/// # use mongo_derive::Bson;
/// use std::convert::TryFrom;
///
/// #[derive(Bson)]
/// struct User {
///     name: String,
///     #[bson(serde)]
///     age: u32,
/// }
///
/// let user = User { name: "foo".to_owned(), age: 0 };
///
/// let bson = mongo::bson::Bson::try_from(user).unwrap();
///
/// println!("{:?}", bson);
/// ```
#[proc_macro_derive(Bson, attributes(bson))]
pub fn derive_bson(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    bson::expand_derive_bson(&input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

/// Derives mongo traits on the decorated type.
///
/// ## Container Attributes
///
/// - `#[mongo(collection = "...")]`: derives the `Collection` trait
/// - `#[mongo(field)]`: derives the `AsField` & `Field` traits
/// - `#[mongo(filter)]`: derives the `AsFilter` & `Filter` traits
/// - `#[mongo(update)]`: derives the `AsUpdate` & `Update` traits
///
/// ### `#[mongo(collection = "...")]`
///
/// Tells the derive to implement the `Collection` trait where the `"..."` is the name of the
/// collection.
///
/// ```
/// # mod wrap {
/// # use mongo_derive::Mongo;
/// # #[derive(mongo_derive::Bson)]
/// #[derive(Mongo)]
/// #[mongo(collection = "users")]
/// pub struct User {
///     name: String,
///     age: u32,
/// }
/// # }
/// ```
///
/// ### `#[mongo(field)]`
///
/// Tells the derive to implement the `AsField` & `Field` traits.
///
/// ```
/// # mod wrap {
/// # use mongo_derive::Mongo;
/// # #[derive(mongo_derive::Bson)]
/// #[derive(Mongo)]
/// #[mongo(field)]
/// pub struct User {
///     name: String,
///     age: u32,
/// }
///
/// // The derived field enum can be exposed from the derived module which uses the type's name in
/// // snake_case
/// use self::user::Field;
/// # }
/// ```
///
/// ### `#[mongo(filter)]`
///
/// Tells the derive to implement the `AsFilter` & `Filter` traits.
///
/// ```
/// # mod wrap {
/// # use mongo_derive::Mongo;
/// # #[derive(mongo_derive::Bson)]
/// #[derive(Mongo)]
/// #[mongo(filter)]
/// pub struct User {
///     name: String,
///     age: u32,
/// }
///
/// // The derived filter struct can be exposed from the derived module which uses the type's name in
/// // snake_case
/// use self::user::Filter;
/// # }
/// ```
///
/// ### `#[mongo(update)]`
///
/// Tells the derive to implement the `AsUpdate` & `Update` traits.
///
/// ```
/// # mod wrap {
/// # use mongo_derive::Mongo;
/// # #[derive(mongo_derive::Bson)]
/// #[derive(Mongo)]
/// #[mongo(update)]
/// pub struct User {
///     name: String,
///     age: u32,
/// }
///
/// // The derived update struct can be exposed from the derived module which uses the type's name in
/// // snake_case
/// use self::user::Update;
/// # }
/// ```
///
/// ## Field Attributes
///
/// - `#[mongo(serde)]`: tells the derive that the field should be handled using serde
/// - `#[mongo(skip)]`: tells the derive to skip the field for `field`, `filter` & `update`
///
/// ### `#[mongo(serde)]`
///
/// Tells the derive that the field should be handled using serde
///
/// ```
/// # mod wrap {
/// # use mongo_derive::Mongo;
/// # #[derive(mongo_derive::Bson)]
/// #[derive(Mongo)]
/// #[mongo(collection = "users")]
/// pub struct User {
///     name: String,
///     #[mongo(serde)]
///     age: u32,
/// }
/// # }
/// ```
///
/// ### `#[mongo(skip)]`
///
/// Tells the derive to skip the field for `field`, `filter` & `update`
///
/// ```
/// # mod wrap {
/// # use mongo_derive::Mongo;
/// # #[derive(mongo_derive::Bson)]
/// #[derive(Mongo)]
/// #[mongo(collection = "users")]
/// pub struct User {
///     name: String,
///     #[mongo(skip)]
///     age: u32,
/// }
/// # }
/// ```
#[proc_macro_derive(Mongo, attributes(mongo))]
pub fn derive_collection(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    mongo::expand_derive_mongo(&input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}
