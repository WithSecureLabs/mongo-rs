mod common;
mod user;

#[cfg(feature = "blocking")]
mod wrapper {
    use super::*;

    use mongod::{AsFilter, AsUpdate, Comparator, Updates};

    use user::User;

    #[test]
    fn blocking_client() {
        common::setup();

        let client = mongod::blocking::Client::new();

        // Insert
        let foo = User {
            name: "foo".to_owned(),
            age: None,
        };
        let bar = User {
            name: "bar".to_owned(),
            age: None,
        };
        let _oid = client.insert::<User>(vec![foo, bar]).unwrap();

        // Fetch
        let mut count: u32 = 0;
        let mut cursor = client.find::<User, _>(None).unwrap();
        while let Some(res) = cursor.next() {
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
        let updated = client.update::<User, _, _>(filter, updates).unwrap();
        assert_eq!(updated, 1);

        // Replace
        let mut filter = User::filter();
        filter.name = Some(Comparator::Eq("bar".to_owned()));
        let foobar = User {
            name: "foobar".to_owned(),
            age: None,
        };
        let _oid = client.replace_one::<User, _>(filter, foobar).unwrap();

        // Delete
        let deleted = client.delete::<User, _>(None).unwrap();
        assert_eq!(deleted, 2);
    }
}
