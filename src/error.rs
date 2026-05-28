#[derive(Debug)]
pub enum ErrorKind {
    General,
    Timeout,
    Abort,
}

#[derive(Debug)]
pub struct Error<E> {
    pub kind: ErrorKind,
    pub inner: E,
    #[cfg(debug_assertions)]
    pub backtrace: std::backtrace::Backtrace,
}

// Error::general(SocketError::Xxx)
impl<E> Error<E> {
    pub fn new(kind: ErrorKind, error: E) -> Self {
        Self {
            kind,
            inner: error,
            #[cfg(debug_assertions)]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }

    #[inline]
    pub fn general(error: E) -> Self {
        Self::new(ErrorKind::General, error)
    }

    #[inline]
    pub fn timeout(error: E) -> Self {
        Self::new(ErrorKind::Timeout, error)
    }

    #[inline]
    pub fn abort(error: E) -> Self {
        Self::new(ErrorKind::Abort, error)
    }
}

impl<E: std::fmt::Display> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.inner)?;
        #[cfg(debug_assertions)]
        writeln!(f, "{}", self.backtrace)?;
        Ok(())
    }
}

macro_rules! impl_from_my_error {
    ($from_inner:ty, $to_inner:ty) => {
        impl From<Error<$from_inner>> for Error<$to_inner> {
            fn from(error: Error<$from_inner>) -> Self {
                Self {
                    kind: error.kind,
                    inner: error.inner.into(),
                    #[cfg(debug_assertions)]
                    backtrace: error.backtrace,
                }
            }
        }
    };
}
