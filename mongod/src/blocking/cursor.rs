use std::marker::PhantomData;

use bson::{oid::ObjectId, Document};
use futures::stream::StreamExt;

use crate::collection::Collection;

#[derive(Debug)]
enum Request {
    Next,
}
enum Response {
    Next(Option<crate::Result<Document>>),
}

/// A blocking version of the [`mongodb::Cursor`](https://docs.rs/mongodb/2.0.0/mongodb/struct.Cursor.html).
///
/// This wraps the async `Cursor` so that is can be called in a synchronous fashion, please see the
/// asynchronous description for more information about the cursor.
pub struct Cursor {
    tx: tokio::sync::mpsc::UnboundedSender<(Request, std::sync::mpsc::Sender<Response>)>,
}

impl Cursor {
    pub(crate) fn new(cursor: mongodb::Cursor<Document>) -> Self {
        let (tx, mut rx) =
            tokio::sync::mpsc::unbounded_channel::<(Request, std::sync::mpsc::Sender<Response>)>();
        let f = async move {
            let mut cursor = cursor;
            while let Some((req, tx)) = rx.recv().await {
                match req {
                    Request::Next => {
                        let resp = cursor
                            .next()
                            .await
                            .map(|n| n.map_err(crate::error::mongodb));
                        let _ = tx.send(Response::Next(resp));
                    }
                };
            }
        };
        tokio::spawn(f);
        Self { tx }
    }
}

impl Iterator for Cursor {
    type Item = crate::Result<Document>;
    fn next(&mut self) -> Option<Self::Item> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.tx
            .send((Request::Next, tx))
            .expect("core thread panicked");
        let res = rx
            .recv()
            .expect("could not get response from mongo runtime");
        let Response::Next(c) = res;
        c
    }
}

/// A typed blocking cursor.
///
/// This wraps the blocking `Cursor` so that is can be automatically return typed documents.
pub struct TypedCursor<T>
where
    T: Collection,
{
    cursor: Cursor,
    document_type: PhantomData<T>,
}

impl<T> TypedCursor<T>
where
    T: Collection,
{
    /// Allow access to the wrapped blocking `Cursor`
    pub fn into_inner(self) -> Cursor {
        self.cursor
    }
}

impl<T> From<Cursor> for TypedCursor<T>
where
    T: Collection,
{
    fn from(cursor: Cursor) -> Self {
        TypedCursor {
            cursor,
            document_type: PhantomData,
        }
    }
}

impl<T> Iterator for TypedCursor<T>
where
    T: Collection,
{
    type Item = crate::Result<(ObjectId, T)>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.cursor.next();

        next.map(|res| {
            let doc = res?;
            let oid = doc.get_object_id("_id").map_err(crate::error::bson)?;
            Ok((oid, T::from_document(doc)?))
        })
    }
}
