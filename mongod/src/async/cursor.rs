use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use bson::Document;
use futures::Stream;
use mongodb::Cursor;

use crate::collection::Collection;

/// A typed blocking cursor.
///
/// This wraps the blocking `Cursor` so that is can be automatically return typed documents.
pub struct TypedCursor<T>
where
    T: Collection,
{
    cursor: Cursor<Document>,
    document_type: PhantomData<T>,
}

impl<T> From<Cursor<Document>> for TypedCursor<T>
where
    T: Collection,
{
    fn from(cursor: Cursor<Document>) -> Self {
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
    type Item = crate::Result<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = Pin::new(&mut self.cursor).poll_next(cx);
        match next {
            Poll::Ready(opt) => Poll::Ready(opt.map(|result| {
                result
                    .map_err(crate::error::mongodb)
                    .and_then(|doc| T::from_document(doc))
            })),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T> Unpin for TypedCursor<T> where T: Collection {}
