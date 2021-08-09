use std::marker::{PhantomData, Unpin};

use bson::Document;
use futures::stream::StreamExt;
use serde::de::DeserializeOwned;

#[derive(Debug)]
enum Request {
    Next,
}
enum Response {
    Next(Option<crate::Result<Document>>),
}

pub(crate) struct CursorInt {
    tx: tokio::sync::mpsc::UnboundedSender<(Request, std::sync::mpsc::Sender<Response>)>,
}

impl CursorInt {
    pub fn new(cursor: mongodb::Cursor<Document>) -> Self {
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

    pub fn to_cursor<T>(self) -> Cursor<T>
    where
        T: DeserializeOwned + Unpin + Send + Sync,
    {
        Cursor {
            document_type: PhantomData,
            tx: self.tx,
        }
    }
}

/// A blocking version of the [`mongodb::Cursor`](https://docs.rs/mongodb/1.1.1/mongodb/struct.Cursor.html).
///
/// This wraps the async `Cursor` so that is can be called in a synchronous fashion, please see the
/// asynchronous description for more information about the cursor.
pub struct Cursor<T>
where
    T: DeserializeOwned + Unpin + Send + Sync,
{
    document_type: PhantomData<T>,

    tx: tokio::sync::mpsc::UnboundedSender<(Request, std::sync::mpsc::Sender<Response>)>,
}

impl<T> Iterator for Cursor<T>
where
    T: DeserializeOwned + Unpin + Send + Sync,
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
            Some(Ok(b)) => Some(bson::from_document::<T>(b).map_err(crate::error::mongodb)),
            Some(Err(e)) => Some(Err(crate::error::mongodb(e))),
            None => None,
        };
        resp
    }
}
