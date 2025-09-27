/// A type which can be written to [`embedded_io_async::Write`] and knows its size.
pub trait Writable {
    type Error<E>;

    /// Returns the amount of bytes [`Self::write_to`] will write when called.
    ///
    /// May return an incorrect value when the data is not valid and can't be written.
    /// The returned may be used for allocations and bounds checks and should in this case,
    /// still be 'reasonable', preferably a lower bound.
    fn size(&self) -> usize;

    /// Writes bytes to a [`sink`](embedded_io_async::Write).
    ///
    /// Implementations must ensure the amount of bytes written matches the size returned from
    /// [`Self::size`].
    // TODO: Send + Sync fun etc.
    fn write_to<S>(&self, sink: S) -> impl Future<Output = Result<(), Self::Error<S::Error>>>
    where
        S: embedded_io_async::Write;
}

impl<T> Writable for Option<T>
where
    T: Writable,
{
    type Error<E> = T::Error<E>;

    fn size(&self) -> usize {
        self.as_ref().map_or(0, |w| w.size())
    }

    async fn write_to<S>(&self, sink: S) -> Result<(), Self::Error<S::Error>>
    where
        S: embedded_io_async::Write,
    {
        if let Some(inner) = &self {
            inner.write_to(sink).await?;
        }
        Ok(())
    }
}

impl<T> Writable for &[T]
where
    T: Writable,
{
    type Error<E> = T::Error<E>;

    fn size(&self) -> usize {
        self.iter().map(|w| w.size()).sum()
    }

    async fn write_to<S>(&self, mut sink: S) -> Result<(), Self::Error<S::Error>>
    where
        S: embedded_io_async::Write,
    {
        for item in self.iter() {
            item.write_to(&mut sink).await?;
        }

        Ok(())
    }
}

// impl<'a> Writable for &'a str {
//     type Error<E> = <EncodedStr<'a> as Writable>::Error<E>;
//
//     fn size(&self) -> usize {
//         EncodedStr(self).size()
//     }
//
//     fn write_to<S>(&self, sink: S) -> impl Future<Output = Result<(), Self::Error<S::Error>>>
//     where
//         S: embedded_io_async::Write,
//     {
//         EncodedStr(self).write_to(sink)
//     }
// }
//
// impl<'a> Writable for &'a [u8] {
//     type Error<E> = <BinaryData<'a> as Writable>::Error<E>;
//
//     fn size(&self) -> usize {
//         BinaryData(self).size()
//     }
//
//     fn write_to<S>(&self, sink: S) -> impl Future<Output = Result<(), Self::Error<S::Error>>>
//     where
//         S: embedded_io_async::Write,
//     {
//         BinaryData(self).write_to(sink)
//     }
// }

macro_rules! impl_writable_be_bytes {
    ($ty:ty) => {
        impl Writable for $ty {
            type Error<E> = E;

            fn size(&self) -> usize {
                self.to_be_bytes().len()
            }

            async fn write_to<S>(&self, mut sink: S) -> Result<(), Self::Error<S::Error>>
            where
                S: embedded_io_async::Write,
            {
                sink.write_all(&self.to_be_bytes()).await
            }
        }
    };
}

impl_writable_be_bytes!(u8);
impl_writable_be_bytes!(u16);
impl_writable_be_bytes!(u32);
