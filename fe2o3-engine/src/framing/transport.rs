use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub struct Transport<T> {
    framed: Framed<T, LengthDelimitedCodec>
}

impl<T: AsyncRead + AsyncWrite> Transport<T> {
    pub fn bind(io: T) -> Self {
        let framed = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            .length_adjustment(4)
            .new_framed(io);
            
        Self { framed }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn test_length_delimited_codec() {
        
    }
}