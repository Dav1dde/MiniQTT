use std::fmt;

pub struct HexBlock<'a> {
    data: &'a [u8],
}

impl<'a> HexBlock<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }
}

impl fmt::Display for HexBlock<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut chunks = self.data.chunks(8).peekable();

        while chunks.peek().is_some() {
            for (i, chunk) in (&mut chunks).take(2).enumerate() {
                if i != 0 {
                    write!(f, "  ")?;
                }

                for j in 0..chunk.len() {
                    if j != 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{byte:02x}", byte = chunk[j])?;
                }
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

pub fn parse_hex_block(data: &str) -> Vec<u8> {
    data.split_whitespace()
        .map(|p| u8::from_str_radix(p, 16))
        .collect::<Result<_, _>>()
        .unwrap()
}
