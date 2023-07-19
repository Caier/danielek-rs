use super::routes::common_types::DiscordApiError;
use serde::{de::DeserializeOwned, Serialize};

pub type Result<T> = std::result::Result<T, DApiError>;

#[derive(Debug)]
pub enum DApiError {
    Instantiation(reqwest::Error),
    Requesting(reqwest::Error),
    ParsingResponse(Box<dyn std::error::Error + Send + Sync + 'static>),
    ApiError(DiscordApiError),
    ApiErrorWithoutBody(reqwest::Error),
}

impl std::fmt::Display for DApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Instantiation(e) => {
                write!(f, "An error occured while trying to create the client: {e}")
            }
            Self::Requesting(e) => write!(f, "Http request error: {e}"),
            Self::ParsingResponse(e) => write!(
                f,
                "An error has occured while trying to parse the http response: {e}"
            ),
            Self::ApiError(e) => write!(f, "Discord API Error: {:#?}", e),
            Self::ApiErrorWithoutBody(e) => write!(f, "Http error: {e}"),
        }
    }
}

impl std::error::Error for DApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Instantiation(e) | Self::Requesting(e) | Self::ApiErrorWithoutBody(e) => Some(e),
            Self::ParsingResponse(e) => Some(&**e),
            Self::ApiError(_) => None,
        }
    }
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
pub trait DApiPATCH<V: DApiVersion>: DApiMethod<V> {
    type Body: Serialize;
    type Response: DeserializeOwned;
}
pub trait DApiDELETE<V: DApiVersion>: DApiMethod<V> {
    type Body: Serialize;
    type Response: DeserializeOwned;
}

macro_rules! dapi_endpoint {
    (   version = $ver:ty
        $(, $meth:tt = ($resp:ty $(, $body:ty)?))+ ;
        $vis:vis fn $name:ident $args:tt $rest:block
    ) => {
        $vis fn $name $args -> impl $($meth<$ver, Response = $resp, $(Body = $body)?> + )+ {
            struct T<E>(E);
            impl<E: AsRef<str>> crate::dapi::types::DApiMethod<$ver> for T<E> {
                fn path(&self) -> &str {
                    self.0.as_ref()
                }
            }
            $(
                impl<E> $meth<$ver> for T<E> where T<E>: crate::dapi::types::DApiMethod<$ver> {
                    type Response = $resp;
                    $(type Body = $body;)?
                }
            )+

            return T($rest);
        }
    };
}
pub(crate) use dapi_endpoint;
