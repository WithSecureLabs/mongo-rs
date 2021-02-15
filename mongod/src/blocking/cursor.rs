use bson::Document;
use futures::stream::StreamExt;

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
    /// Constructs a synchronous `Cursor` from an asynchronous one.
    pub fn new(cursor: mongodb::Cursor) -> Self {
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
