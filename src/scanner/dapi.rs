use std::{time::Duration, fmt::Display, marker::PhantomData};
use once_cell::sync::Lazy;
use reqwest::Method;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::error::Error as StdError;

type Result<T> = std::result::Result<T, DApiError>;

enum DApiError {
    Instantiation(reqwest::Error),
    Serialization(serde_json::Error),
    Requesting(reqwest::Error)
}

trait DApiMethod<V: DApiVersion> {
    fn path(&self) -> &str;
}

trait DApiGET<V: DApiVersion>: DApiMethod<V> {
    type Response: DeserializeOwned;
}
trait DApiPOST<V: DApiVersion>: DApiMethod<V> {
    type Body: Serialize;
    type Response: DeserializeOwned;
}
trait DApiPUT<V: DApiVersion>: DApiMethod<V> {
    type Body: Serialize;
    type Response: DeserializeOwned;
}

macro_rules! dapi_endpoint {
    ($ver:ty, $path:expr $(, $meth:tt => ($resp:ty $(, $body:ty)?))+) => {
        struct DApiEndpointAnonImpl<E>(E);
        impl<E: AsRef<str>> DApiMethod<$ver> for DApiEndpointAnonImpl<E> {
            fn path(&self) -> &str {
                self.0.as_ref()
            }
        }
        $(
            impl<E> $meth<$ver> for DApiEndpointAnonImpl<E> where DApiEndpointAnonImpl<E>: DApiMethod<$ver> {
                type Response = $resp;
                $(type Body = $body;)?
            } 
        )+

        return DApiEndpointAnonImpl($path);
    }
}

fn guild_channels() -> impl DApiGET<v10> + DApiPOST<v10> {
    dapi_endpoint!(v10, "/channels", 
        DApiPOST => ((), ()), 
        DApiGET => (())
    );
}

#[derive(Debug, Serialize)]
struct MessagePayload {
    content: Option<String>,
    nonce: Option<String>,
    tts: Option<bool>
}

trait DApiVersion {
    const VER: &'static str;
}

struct v10;

impl DApiVersion for v10 {
    const VER: &'static str = "v10";
}

struct DApi<V: DApiVersion> {
    http: reqwest::Client,
    token: String,
    api_base: String,
    api_ver: PhantomData<V>
}

impl<V: DApiVersion> DApi<V> {
    pub fn new(token: impl Into<String>) -> Result<Self> {
        Ok(Self {
            http: reqwest::ClientBuilder::new().timeout(Duration::from_secs(5)).build()
                .map_err(|e| DApiError::Instantiation(e))?,
            token: format!("{}", token.into()),
            api_base: format!("https://discord.com/api/{}/", V::VER),
            api_ver: PhantomData
        })
    }

    async fn request<R: DeserializeOwned, B: Serialize>(&self, method: Method, path: &str, body: Option<B>) -> Result<R> {
        let mut req = self.http.request(method, path)
            .header("Authorization", &self.token)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Safari/537.36");

        if let Some(b) = body {
            req = req.body(serde_json::to_string(&b).map_err(|e| DApiError::Serialization(e))?);
        }

        let resp = req.send().await
            .and_then(|r| r.error_for_status())
            .map_err(|e| DApiError::Requesting(e))?;

        Ok(resp.json().await.map_err(|e| DApiError::Requesting(e))?)
    }

    pub async fn get<T: DApiGET<V>>(&self, route: T) -> Result<T::Response> {
        self.request(Method::GET, route.path(), None as Option<()>).await
    }
}