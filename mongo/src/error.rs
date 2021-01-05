use std::error::Error as StdError;
use std::fmt;

/// A `Result` alias where `Err` case is `mongo::Error`.
pub type Result<T> = std::result::Result<T, Error>;

/// The errors that may occur when talking to mongo.
pub struct Error {
    inner: Box<Inner>,
}

pub(crate) type Source = Box<dyn StdError + Send + Sync>;

struct Inner {
    kind: Kind,
    source: Option<Source>,
}

impl Error {
    pub(crate) fn new(kind: Kind) -> Error {
        Error {
            inner: Box::new(Inner { kind, source: None }),
        }
    }

    pub(crate) fn with<S: Into<Source>>(mut self, source: S) -> Error {
        self.inner.source = Some(source.into());
        self
    }

    /// Returns the kind of this error.
    ///
    /// # Examples
    ///
    /// ```
    /// use mongo::Client;
    /// use mongo::ErrorKind;
    ///
    /// fn run() {
    ///     if let Err(e) = mongo::Client::builder().build() {
    ///         match e.kind() {
    ///             ErrorKind::Builder => println!("we have a builder error..."),
    ///             _ => {},
    ///         }
    ///     }
    /// }
    /// ```
    pub fn kind(&self) -> &Kind {
        &self.inner.kind
    }

    /// Creates a custom `Kind::InvalidDocument` error.
    ///
    /// This is useful when manually implementating `mongo` traits.
    pub fn invalid_document<E: Into<Source>>(error: E) -> Error {
        Error::new(Kind::InvalidDocument).with(error)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = fmt.debug_struct("mongo::Error");
        builder.field("kind", &self.inner.kind);
        if let Some(ref source) = self.inner.source {
            builder.field("source", source);
        }
        builder.finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let desc = match self.inner.kind {
            Kind::Bson => "bson error",
            Kind::Builder => "builder error",
            Kind::Mongodb => "mongodb error",
            Kind::InvalidDocument => "invalid document",
            Kind::Runtime => "runtime error",
        };
        if let Some(ref source) = self.inner.source {
            write!(f, "{}: {}", desc, source)
        } else {
            f.write_str(&desc)
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.inner.source.as_ref().map(|e| &**e as _)
    }
}

impl From<crate::ext::bson::de::Error> for Error {
    fn from(de: crate::ext::bson::de::Error) -> Self {
        Error::invalid_document(de)
    }
}

impl From<crate::ext::bson::ser::Error> for Error {
    fn from(ser: crate::ext::bson::ser::Error) -> Self {
        Error::invalid_document(ser)
    }
}

/// The `Kind` of `mongo::Error`.
#[derive(Debug)]
pub enum Kind {
    Bson,
    Builder,
    Mongodb,
    Runtime,
    InvalidDocument,
}

// Helpers
pub(crate) fn bson<E: Into<Source>>(e: E) -> Error {
    Error::new(Kind::Bson).with(e)
}

pub(crate) fn builder<E: Into<Source>>(e: E) -> Error {
    Error::new(Kind::Builder).with(e)
}

pub(crate) fn mongodb<E: Into<Source>>(e: E) -> Error {
    Error::new(Kind::Mongodb).with(e)
}

#[cfg(feature = "blocking")]
pub(crate) fn runtime<E: Into<Source>>(e: E) -> Error {
    Error::new(Kind::Runtime).with(e)
}
