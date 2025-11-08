use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(doc)]
use crate::client::Client;
use crate::client::Result;
use crate::protocol::v5;

pin_project_lite::pin_project! {
    /// Future returned by [`Client::connect`].
    ///
    /// # Cancel safety
    ///
    /// This future is *not* cancel safe.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Connect<'a, T> {
        packet: v5::Connect<'a>,
        #[pin]
        inner: T,
    }
}

impl<'a, T> Connect<'a, T> {
    pub fn keep_alive(mut self, keep_alive: u16) -> Self {
        self.packet.keep_alive = keep_alive;
        self
    }

    pub fn resume_session(mut self, resume: bool) -> Self {
        self.packet.clean_start = !resume;
        self
    }

    pub fn with_username<S>(mut self, username: S) -> Self
    where
        S: Into<Option<&'a str>>,
    {
        self.packet.username = username.into();
        self
    }

    pub fn with_password<S>(mut self, password: S) -> Self
    where
        S: Into<Option<&'a str>>,
    {
        self.packet.password = password.into();
        self
    }

    pub fn with_properties(mut self, properties: &'a [v5::connect::ConnectProperty<'a>]) -> Self {
        self.packet.properties = properties;
        self
    }
}

impl Connect<'_, ()> {
    pub(super) fn new<'a, M, F, E>(
        client_id: &'a str,
        m: M,
    ) -> Connect<'a, impl MakeFuture<'a, Error = E>>
    where
        M: FnOnce(v5::Connect<'a>) -> F,
        F: Future<Output = Result<(), E>>,
    {
        let packet = v5::Connect {
            client_id,
            keep_alive: 0,
            clean_start: true,
            will: None,
            username: None,
            password: None,
            properties: &[],
        };

        Connect {
            packet,
            inner: Inner::Make { make: Some(m) },
        }
    }
}

impl<'a, M> Future for Connect<'a, M>
where
    M: MakeFuture<'a>,
{
    type Output = Result<(), M::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner.poll(this.packet, cx)
    }
}

pub trait MakeFuture<'a> {
    type Error;

    fn poll(
        self: Pin<&mut Self>,
        state: &v5::Connect<'a>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>>;
}

pin_project_lite::pin_project! {
    #[project = InnerProj]
    enum Inner<M, F> {
        Make { make: Option<M> },
        Future { #[pin] future: F },
    }
}

impl<'a, M, F, E> MakeFuture<'a> for Inner<M, F>
where
    M: FnOnce(v5::Connect<'a>) -> F,
    F: Future<Output = Result<(), E>>,
{
    type Error = E;

    fn poll(
        mut self: Pin<&mut Self>,
        state: &v5::Connect<'a>,
        cx: &mut Context<'_>,
    ) -> Poll<F::Output> {
        loop {
            match self.as_mut().project() {
                InnerProj::Make { make } => {
                    // This will only panic if `make` panic'd before
                    let make = make.take().unwrap();
                    let future = (make)(*state);
                    self.set(Inner::Future { future });
                }
                InnerProj::Future { future } => break future.poll(cx),
            }
        }
    }
}
