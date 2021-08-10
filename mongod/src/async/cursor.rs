use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use bson::{oid::ObjectId, Document};
use futures::Stream;

use crate::collection::Collection;

/// A typed cursor.
///
/// This wraps the `Cursor` so that it can be automatically return typed documents.
pub struct TypedCursor<T>
where
    T: Collection,
{
    cursor: mongodb::Cursor<Document>,
    document_type: PhantomData<T>,
}

impl<T> TypedCursor<T>
where
    T: Collection,
{
    /// Allow access to the wrapped [`mongodb::Cursor`](https://docs.rs/mongodb/2.0.0/mongodb/struct.Cursor.html).
    pub fn into_inner(self) -> mongodb::Cursor<Document> {
        self.cursor
    }
}

impl<T> From<mongodb::Cursor<Document>> for TypedCursor<T>
where
    T: Collection,
{
    fn from(cursor: mongodb::Cursor<Document>) -> Self {
        TypedCursor {
            cursor: cursor,
            document_type: PhantomData,
        }
    }
}

impl<T> Stream for TypedCursor<T>
where
    T: Collection,
{
    type Item = crate::Result<(ObjectId, T)>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = Pin::new(&mut self.cursor).poll_next(cx);
        match next {
            Poll::Ready(opt) => Poll::Ready(opt.map(|result| {
                let doc = result.map_err(crate::error::mongodb)?;
                let oid = doc.get_object_id("_id").map_err(crate::error::bson)?;
                Ok((oid, T::from_document(doc)?))
            })),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> Unpin for TypedCursor<T> where T: Collection {}
