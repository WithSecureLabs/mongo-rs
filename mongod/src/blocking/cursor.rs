use std::marker::PhantomData;

use bson::Document;
use futures::stream::StreamExt;

use crate::collection::Collection;

#[derive(Debug)]
enum Request {
    Next,
}
enum Response {
    Next(Option<crate::Result<Document>>),
}

/// A blocking version of the [`mongodb::Cursor`](https://docs.rs/mongodb/1.1.1/mongodb/struct.Cursor.html).
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
    document_type: PhantomData<T>,

    tx: tokio::sync::mpsc::UnboundedSender<(Request, std::sync::mpsc::Sender<Response>)>,
}

impl<T> From<Cursor> for TypedCursor<T>
where
    T: Collection,
{
    fn from(cursor: Cursor) -> Self {
        TypedCursor {
            document_type: PhantomData,
            tx: cursor.tx,
        }
    }
}

impl<T> Iterator for TypedCursor<T>
where
    T: Collection,
{
    type Item = crate::Result<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.tx
            .send((Request::Next, tx))
            .expect("core thread panicked");
        let res = rx
            .recv()
            .expect("could not get response from mongo runtime");
        let Response::Next(c) = res;
        let resp = match c {
            Some(Ok(b)) => Some(T::from_document(b)),
            Some(Err(e)) => Some(Err(crate::error::mongodb(e))),
            None => None,
        };
        resp
    }
}
