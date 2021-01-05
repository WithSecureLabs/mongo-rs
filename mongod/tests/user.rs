use std::convert::{TryFrom, TryInto};

use mongod::bson::{Bson, Document};
use mongod::ext;
use mongod::{AsField, AsFilter, AsUpdate, Collection, Comparator, Error, Field, Filter, Update};

pub struct User {
    pub name: String,
    pub age: Option<u32>,
}

impl Collection for User {
    const COLLECTION: &'static str = "users";

    fn from_document(document: Document) -> Result<Self, Error> {
        let mut document = document;
        let mut name: Option<String> = None;
        let mut age: Option<Option<u32>> = None;
        if let Some(value) = document.remove("name") {
            name = Some(String::try_from(ext::bson::Bson(value))?);
        }
        if let Some(value) = document.remove("age") {
            let wrap = ext::bson::Bson(value);
            let opt = match Option::<Bson>::from(wrap) {
                Some(v) => Some(
                    ext::bson::Bson(v)
                        .try_into()
                        .map_err(Error::invalid_document)?,
                ),
                None => None,
            };
            age = Some(opt)
        }
        if name.is_none() {
            return Err(Error::invalid_document("missing required field `name`"));
        }
        if age.is_none() {
            return Err(Error::invalid_document("missing required field `age`"));
        }
        Ok(Self {
            name: name.expect("could not get name"),
            age: age.expect("could not get name"),
        })
    }

    fn into_document(self) -> Result<Document, Error> {
        let mut doc = Document::new();
        doc.insert("name", self.name);
        doc.insert("age", ext::bson::Bson::try_from(self.age)?.0);
        Ok(doc)
    }
}

impl AsField<UserField> for User {}

pub enum UserField {
    Name,
    Age,
}

impl Field for UserField {}

impl From<UserField> for String {
    fn from(field: UserField) -> String {
        match field {
            UserField::Name => "name".to_owned(),
            UserField::Age => "age".to_owned(),
        }
    }
}

#[derive(Default)]
pub struct UserFilter {
    pub name: Option<Comparator<String>>,
    pub age: Option<Comparator<Option<u32>>>,
}

impl Filter for UserFilter {
    fn new() -> Self {
        Self::default()
    }

    fn into_document(self) -> Result<Document, Error> {
        let mut doc = Document::new();
        if let Some(value) = self.name {
            doc.insert("name", ext::bson::Bson::try_from(value)?.0);
        }
        if let Some(value) = self.age {
            doc.insert("age", ext::bson::Bson::try_from(value)?.0);
        }
        Ok(doc)
    }
}

impl AsFilter<UserFilter> for User {
    fn filter() -> UserFilter {
        UserFilter::default()
    }

    fn into_filter(self) -> UserFilter {
        UserFilter {
            name: Some(Comparator::Eq(self.name)),
            age: Some(Comparator::Eq(self.age)),
        }
    }
}

#[derive(Default)]
pub struct UserUpdate {
    pub name: Option<String>,
    pub age: Option<Option<u32>>,
}

impl Update for UserUpdate {
    fn new() -> Self {
        UserUpdate::default()
    }
    fn into_document(self) -> Result<Document, Error> {
        let mut doc = Document::new();
        if let Some(value) = self.name {
            doc.insert("name", value);
        }
        if let Some(value) = self.age {
            doc.insert("age", ext::bson::Bson::try_from(value)?.0);
        }
        Ok(doc)
    }
}

impl AsUpdate<UserUpdate> for User {
    fn update() -> UserUpdate {
        UserUpdate::default()
    }
    fn into_update(self) -> UserUpdate {
        UserUpdate {
            name: Some(self.name),
            age: Some(self.age),
        }
    }
}
