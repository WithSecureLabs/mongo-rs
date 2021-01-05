# Mongo

[![crates.io](https://img.shields.io/crates/v/mongod.svg)](https://crates.io/crates/mongod)
[![Documentation](https://docs.rs/mongod/badge.svg)](https://docs.rs/mongod)

A higher-level wrapper on top of the official bson & mongodb crates.

## Overview

The `mongod` crate is an unofficial higher-level wrapper that aims to make the bson & mongodb crates a little bit more rust-like.

Provides the following:
- builder style queries
- clients with async/non-async support
- derives for `Bson` & `Mongo` to help with BSON type conversion and implentation of traits provided by this crate
- strong opinions through the use of traits

## Example

```toml
futures = "0.3"
mongod = { version = "0.1", features = ["derive"] }
tokio = { version = "0.2", features = ["full"] }
```

```rust
use futures::stream::StreamExt;
use mongod::{Bson, Mongo};
use mongod::{AsFilter, AsUpdate, Collection, Comparator, Updates};

#[derive(Debug, Bson, Mongo)]
#[mongo(collection = "users", field, filter, update)]
pub struct User {
  pub name: String,
  pub age: Option<u32>,
}

#[tokio::main]
async fn main() {
  // Create and async client
  let client = mongod::Client::new();

  // Insert a user into the users collection
  let user = User { name: "foo".to_string(), age: None };
  let oid = client.insert_one::<User>(user).await.unwrap();
  println!("{:?}", oid);

  // Fetch all users in the users collection
  let mut cursor = client.find::<User, _>(None).await.unwrap();
  while let Some(res) = cursor.next().await {
      if let Ok(doc) = res {
          let user: User = User::from_document(doc).unwrap();
          println!("{:?}", user);
      }
  }

  // Update the user
  let mut filter = User::filter();
  filter.name = Some(Comparator::Eq("foo".to_owned()));
  let mut update = User::update();
  update.age = Some(Some(0));
  let updates = Updates {
      set: Some(update),
      ..Default::default()
  };
  let updated = client.update::<User, _, _>(filter, updates).await.unwrap();
  println!("updated {} documents", updated);

  // Delete all users
  let deleted = client.delete::<User, _>(None).await.unwrap();
  println!("delete {} documents", deleted);
}
```

## Client

### Async

The default client is async and provides convenience functions over those exposed by the mongodb driver.

Example: see above.

### Blocking

Not everything should be async and for that reason a blocking client is provided that can be used at the same time as the async client.

```toml
mongod = { version = "0.1", features = ["blocking", "derive"] }
```

```rust
use mongod::{Bson, Mongo};
use mongod::{AsFilter, AsUpdate, Collection, Comparator, Updates};

#[derive(Debug, Bson, Mongo)]
#[mongo(collection = "users", field, filter, update)]
pub struct User {
  pub name: String,
  pub age: Option<u32>,
}

fn main() {
  // Create and async client
  let client = mongo::blocking::Client::new();

  // Insert a user into the users collection
  let user = User { name: "foo".to_string(), age: None };
  let oid = client.insert_one::<User>(user).unwrap();
  println!("{:?}", oid);

  // Fetch all users in the users collection
  let mut cursor = client.find::<User, _>(None).unwrap();
  while let Some(res) = cursor.next() {
      if let Ok(doc) = res {
          let user: User = User::from_document(doc).unwrap();
          println!("{:?}", user);
      }
  }

  // Update the user
  let mut filter = User::filter();
  filter.name = Some(Comparator::Eq("foo".to_owned()));
  let mut update = User::update();
  update.age = Some(Some(0));
  let updates = Updates {
      set: Some(update),
      ..Default::default()
  };
  let updated = client.update::<User, _, _>(filter, updates).unwrap();
  println!("updated {} documents", updated);

  // Delete all users
  let deleted = client.delete::<User, _>(None).unwrap();
  println!("delete {} documents", deleted);
}
```

## Complex Queries

Sometimes the convenience queries provided on the client are not enough, the query builders can be used instead.

```rust
use mongod::Query;

...

let result = Query::insert::<User>::new()
    .document_validation(false)
    .ordered(false)
    .query(&client, vec![user])
    .await
    .unwrap();

...
```

## Serde

Sometimes there are reasons that implenting `TryFrom` is just too difficult, but serde implentations already exist.
By tweaking the `Mongo` derive it can be changed to be serde backed.

```rust
use mongod::Mongo;

#[derive(Debug, Mongo)]
#[mongo(bson = "serde", collection = "users", field, filter, update)]
pub struct User {
  pub name: String,
  pub age: Option<u32>,
}

...
```

## Too Many Opinions

This library is too opinionated but I wanna use the derives...
Well as the derives are basically just fancy ways to convert rust types into BSON, they can be used without the `mongod` clients.
Below is an example of the `mongodb` client but using a `mongod` derived user.

```rust
use mongod::{Bson, Mongo};
use mongod::Collection;
use mongod::db::Client;

#[derive(Debug, Bson, Mongo)]
#[mongo(collection = "users", field, filter, update)]
pub struct User {
  pub name: String,
}

let client = Client::with_uri_str("mongodb://localhost:27017/").await.unwrap();
let db = client.database("db");
let users = db.collection("users");
let user = User { name: "foo".to_owned() };
let result = users.insert_one(user.into_document().unwrap(), None).unwrap();
```

## TODO

- Add proc macro tests
- Not all features have been implented yet...
