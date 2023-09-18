pub extern crate bincode;
pub extern crate bytes;
pub extern crate tokio_util;

#[macro_export]
macro_rules! codec {
    (
    $codec:ident,
    encode: $en_item:ident $(<$($en_generic:tt),+>)?,
    decode: $de_item:ident $(<$($de_generic:tt),+>)?
    ) => {
        $crate::codec_struct!($codec $(<$($de_generic),+>)?);

        $crate::encoder! {@impl
            $en_item $(<$($en_generic),+>)?,
            $codec $(<$($de_generic),+>)?
        }

        $crate::decoder! {@impl
            $de_item $(<$($de_generic),+>)?,
            $codec
        }

    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! codec_struct {
    ($codec:ident $(<$($generic:tt),+>)? ) => {
        pub struct $codec $(<$($generic),+>)?{
            len_codec: $crate::macros::codec::tokio_util::codec::LengthDelimitedCodec,
            $(phamtom: std::marker::PhantomData<$($generic),+>)?
        }

        impl$(<$($generic),+>)? $codec $(<$($generic),+>)? {
            pub fn new() -> Self {
                Self {
                    len_codec: $crate::macros::codec::tokio_util::codec::LengthDelimitedCodec::new(),
                    $(phamtom: std::marker::PhantomData::<$($generic),+>)?
                }
            }
        }
    };
}

#[macro_export]
macro_rules! encoder {
    ($item:ident $(<$($tt:tt),+>)? $(,)?) => {
        $crate::encoder!($item $(<$($tt),+>)?, MsgEncoder);
    };

    (item: $item:ident $(<$($tt:tt),+>)?, codec: $codec:ident) => {
        $crate::macros::codec_struct!($codec);
        $crate::encoder!(@impl $item $(<$($tt)+>)?, $codec);
    };

    (@impl $item:ident, $codec:ident  $(<$($de_generic:tt),+>)?) => {
        const _: () = {
            use $crate::macros::codec::bytes::BytesMut;
            use $crate::macros::codec::tokio_util::codec::Encoder;

            impl$(<$($de_generic)+>)? Encoder<$item> for $codec $(<$($de_generic)+>)?
            {
                type Error = anyhow::Error;

                fn encode(&mut self, item: $item, dst: &mut BytesMut) -> Result<(), Self::Error> {
                    let msg = $crate::macros::codec::bincode::serialize(&item)?;
                    self.len_codec.encode(msg.into(), dst)?;
                    Ok(())
                }
            }
        };
    };

    (@impl $item:ident $(<$($tt:tt),+>)?, $codec:ident) => {
        const _: () = {
            use $crate::macros::codec::bytes::BytesMut;
            use $crate::macros::codec::tokio_util::codec::Encoder;

            impl$(<$($tt),+ >)? Encoder<$item $(<$($tt),+>)? > for $codec
                $(
                where
                    $($tt: ::serde::Serialize,)+
                )?
            {
                type Error = anyhow::Error;

                fn encode(&mut self, item: $item $(<$($tt),+>)?, dst: &mut BytesMut) -> Result<(), Self::Error> {
                    let msg = $crate::macros::codec::bincode::serialize(&item)?;
                    self.len_codec.encode(msg.into(), dst)?;
                    Ok(())
                }
            }
        };
    };

    (@impl $item:ident $(<$($tt:tt),+>)?, $codec:ident  $(<$($de_generic:tt),+>)?) => {
        const _: () = {
            use $crate::macros::codec::bytes::BytesMut;
            use $crate::macros::codec::tokio_util::codec::Encoder;

            impl$(<$($tt),+ , $($de_generic)+>)? Encoder<$item $(<$($tt),+>)? > for $codec $(<$($de_generic)+>)?
                $(
                where
                    $($tt: ::serde::Serialize,)+
                )?
            {
                type Error = anyhow::Error;

                fn encode(&mut self, item: $item $(<$($tt),+>)?, dst: &mut BytesMut) -> Result<(), Self::Error> {
                    let msg = $crate::macros::codec::bincode::serialize(&item)?;
                    self.len_codec.encode(msg.into(), dst)?;
                    Ok(())
                }
            }
        };
    };
}

#[macro_export]
macro_rules! decoder {
    ($item:ident $(<$($tt:tt),+>)? $(,)?) => {
        $crate::decoder!($item $(<$($tt),+>)?, MsgEncoder);
    };

    ($item:ident $(<$($tt:tt),+>)?, $codec:ident) => {
        $crate::macros::codec_struct!($codec $(<$($tt)+>)?);
        $crate::decoder!(@impl $item $(<$($tt)+>)?, $codec);
    };

    (@impl $item:ident $(<$($tt:tt),+>)?, $codec:ident) => {
        const _: () = {
            use $crate::macros::codec::bytes::BytesMut;
            use $crate::macros::codec::tokio_util::codec::Decoder;

            impl$(<$($tt)+>)? Decoder for $codec $(<$($tt)+>)?
                $(
                where
                    $($tt: for<'a> ::serde::Deserialize<'a>,)+
                )?

            {
                type Item = $item $(<$($tt)+>)?;

                type Error = anyhow::Error;

                fn decode(
                    &mut self,
                    src: &mut BytesMut,
                ) -> Result<Option<Self::Item>, Self::Error> {
                    if let Some(bytes) = self.len_codec.decode(src)? {
                        let msg = $crate::macros::codec::bincode::deserialize(&*bytes)?;
                        Ok(Some(msg))
                    } else {
                        Ok(None)
                    }
                }
            }
        };
    };
}

mod test {

    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct Req<R> {
        msg: R,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Resp<T> {
        msg: T,
    }

    codec!(Codec, encode: Req<R>, decode: Resp<T>);

    use tokio_util::codec::Encoder;

    #[allow(unused)]
    fn aa<En, R>(_: En)
    where
        R: serde::Serialize,
        En: Encoder<Req<R>, Error = anyhow::Error>,
    {
    }

    #[allow(unused)]
    fn bb<T>()
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        let codec = Codec::<T>::new();
        aa::<_, u8>(codec);
    }
}
