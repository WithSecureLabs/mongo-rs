/// Used to mark an `enum` as a viable type for use in sorting.
///
/// # Example
///
/// Defining an `enum` as a set of fields for use in a mongo query.
///
/// ```
/// # mod wrapper {
/// # use mongod_derive::{Bson, Mongo};
/// use mongod::Field;
///
/// #[derive(Bson, Mongo)]
/// #[mongo(collection="users")]
/// pub struct User {
///     pub name: String,
/// }
///
/// pub enum UserField {
///     Name,
/// }
///
/// impl Field for UserField {}
///
/// impl From<UserField> for String {
///     fn from(field: UserField) -> String {
///         match field {
///             UserField::Name => "name".to_owned(),
///         }
///     }
/// }
/// # }
/// ```
pub trait Field {}

/// Used to tie a type implementing [`Collection`](./trait.Collection.html) to its companion `Field` type.
///
/// # Example
///
/// Defining an `enum` as a set of fields for use in a mongo query.
///
/// ```
/// # mod wrapper {
/// # use mongod_derive::{Bson, Mongo};
/// use mongod::{AsField, Field};
///
/// #[derive(Bson, Mongo)]
/// #[mongo(collection="users")]
/// pub struct User {
///     pub name: String,
/// }
///
/// impl AsField<UserField> for User {}
///
/// pub enum UserField {
///     Name,
/// }
///
/// impl Field for UserField {}
///
/// impl From<UserField> for String {
///     fn from(field: UserField) -> String {
///         match field {
///             UserField::Name => "name".to_owned(),
///         }
///     }
/// }
/// # }
/// ```
pub trait AsField<F: Field + Into<String>> {}
