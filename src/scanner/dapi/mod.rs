use std::{marker::PhantomData, time::Duration};

use reqwest::Method;
use serde::{de::DeserializeOwned, Serialize};

use self::types::{Result, DApiVersion, DApiError, DApiGET, DApiPOST};

mod types;
pub mod routes;

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

    pub async fn post<T: DApiPOST<V>>(&self, route: T, body: T::Body) -> Result<T::Response> {
        self.request(Method::POST, route.path(), Some(body)).await
    }
}