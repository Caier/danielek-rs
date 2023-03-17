use serde::{Serialize, de::DeserializeOwned};

pub type Result<T> = std::result::Result<T, DApiError>;

pub enum DApiError {
    Instantiation(reqwest::Error),
    Serialization(serde_json::Error),
    Requesting(reqwest::Error)
}

pub trait DApiVersion {
    const VER: &'static str;
}

pub trait DApiMethod<V: DApiVersion> {
    fn path(&self) -> &str;
}

pub trait DApiGET<V: DApiVersion>: DApiMethod<V> {
    type Response: DeserializeOwned;
}
pub trait DApiPOST<V: DApiVersion>: DApiMethod<V> {
    type Body: Serialize;
    type Response: DeserializeOwned;
}
pub trait DApiPUT<V: DApiVersion>: DApiMethod<V> {
    type Body: Serialize;
    type Response: DeserializeOwned;
}

macro_rules! dapi_endpoint {
    (   version = $ver:ty
        $(, $meth:tt = ($resp:ty $(, $body:ty)?))+ ; 
        $vis:vis fn $name:ident $args:tt $rest:block
    ) => {
        $vis fn $name $args -> impl $($meth<$ver> + )+ {
            struct T<E>(E);
            impl<E: AsRef<str>> DApiMethod<$ver> for T<E> {
                fn path(&self) -> &str {
                    self.0.as_ref()
                }
            }
            $(
                impl<E> $meth<$ver> for T<E> where T<E>: DApiMethod<$ver> {
                    type Response = $resp;
                    $(type Body = $body;)?
                } 
            )+

            return T($rest);
        }
    };
}
pub(crate) use dapi_endpoint;