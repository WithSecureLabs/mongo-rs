use futures::stream::StreamExt;
use mongod::{AsFilter, AsUpdate, Comparator, Updates};

use user::User;

mod common;
mod user;

#[tokio::test]
async fn async_client() {
    common::async_setup().await;

    let client = mongod::Client::new();

    // Insert
    let foo = User {
        name: "foo".to_owned(),
        age: None,
    };
    let bar = User {
        name: "bar".to_owned(),
        age: None,
    };
    let _oid = client.insert::<User>(vec![foo, bar]).await.unwrap();

    // Fetch
    let mut count: u32 = 0;
    let mut cursor = client.find::<User, _>(None).await.unwrap();
    while let Some(res) = cursor.next().await {
        let (_id, _user) = res.unwrap();
        count += 1;
    }
    assert_eq!(count, 2);

    // Update
    let mut filter = User::filter();
    filter.name = Some(Comparator::Eq("foo".to_owned()));
    let mut set = User::update();
    set.age = Some(Some(1_000_000));
    let updates = Updates {
        set: Some(set),
        ..Default::default()
    };
    let updated = client.update::<User, _, _>(filter, updates).await.unwrap();
    assert_eq!(updated, 1);

    // Replace
    let mut filter = User::filter();
    filter.name = Some(Comparator::Eq("bar".to_owned()));
    let foobar = User {
        name: "foobar".to_owned(),
        age: None,
    };
    let _oid = client.replace_one::<User, _>(filter, foobar).await.unwrap();

    // Delete
    let deleted = client.delete::<User, _>(None).await.unwrap();
    assert_eq!(deleted, 2);
}
