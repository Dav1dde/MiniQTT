use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait MakeFuture<S> {
    type Output;

    fn poll(self: Pin<&mut Self>, state: &S, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

pin_project_lite::pin_project! {
    #[project = LazyMakeFutureProj]
    pub(super) enum LazyMakeFuture<M, F, S> {
        Make { make: Option<M>, _phantom: PhantomData<S> },
        Future { #[pin] future: F },
    }
}

impl<M, F, S> LazyMakeFuture<M, F, S> {
    pub fn new(make: M) -> Self {
        Self::Make {
            make: Some(make),
            _phantom: Default::default(),
        }
    }
}

impl<M, S, F, O> MakeFuture<S> for LazyMakeFuture<M, F, S>
where
    M: FnOnce(S) -> F,
    F: Future<Output = O>,
    S: Copy,
{
    type Output = O;

    fn poll(mut self: Pin<&mut Self>, state: &S, cx: &mut Context<'_>) -> Poll<F::Output> {
        loop {
            match self.as_mut().project() {
                LazyMakeFutureProj::Make { make, .. } => {
                    // This will only panic if `make` panic'd before
                    let make = make.take().unwrap();
                    let future = (make)(*state);
                    self.set(LazyMakeFuture::Future { future });
                }
                LazyMakeFutureProj::Future { future } => break future.poll(cx),
            }
        }
    }
}
