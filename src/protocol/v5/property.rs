use crate::{protocol::types::VariableByteInteger, traits::Writable};

pub trait Property {}

pub struct Properties<'a, T>(pub &'a [T]);

impl<T> Writable for Properties<'_, T>
where
    T: Writable,
{
    type Error<E> = T::Error<E>;

    fn size(&self) -> usize {
        let s = self.0.size();
        VariableByteInteger::try_from(s).ok().size() + s
    }

    async fn write_to<S>(&self, mut sink: S) -> Result<(), Self::Error<S::Error>>
    where
        S: embedded_io_async::Write,
    {
        // TODO: error handling
        VariableByteInteger::try_from(self.0.size())
            .unwrap()
            .write_to(&mut sink)
            .await
            .unwrap();

        self.0.write_to(&mut sink).await?;

        Ok(())
    }
}
