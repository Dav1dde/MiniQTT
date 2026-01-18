/// Returned by [`Buffer::try_resize`] if the buffer is not resizable.
#[derive(Debug, Clone, Copy)]
pub struct BufferNotResizable;

/// A continuous buffer, which may be resizable.
pub trait Buffer {
    /// Returns the entire contents of the buffer as continuous byte slice.
    fn as_slice(&self) -> &[u8];

    /// Returns the entire contents of the buffer as continuous mutable byte slice.
    fn as_slice_mut(&mut self) -> &mut [u8];

    /// Attempts to resize the buffer.
    ///
    /// By default a buffer is not resizable and this returns [`BufferNotResizable`].
    ///
    /// Implementations must either return [`BufferNotResizable`] or extend the buffer
    /// by at least one byte.
    fn try_resize(&mut self) -> Result<(), BufferNotResizable> {
        Err(BufferNotResizable)
    }
}

impl Buffer for &mut [u8] {
    fn as_slice(&self) -> &[u8] {
        self
    }

    fn as_slice_mut(&mut self) -> &mut [u8] {
        self
    }
}

impl Buffer for Vec<u8> {
    fn as_slice(&self) -> &[u8] {
        self
    }

    fn as_slice_mut(&mut self) -> &mut [u8] {
        self
    }

    fn try_resize(&mut self) -> Result<(), BufferNotResizable> {
        let len = self.len();
        self.resize(len + (len * 2).clamp(32, 8192), 0);
        Ok(())
    }
}

impl<const N: usize> Buffer for [u8; N] {
    fn as_slice(&self) -> &[u8] {
        self
    }

    fn as_slice_mut(&mut self) -> &mut [u8] {
        self
    }
}

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
