#![allow(unused)]

use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use lazy_regex::regex;
use reqwest::{Method, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use tokio::{sync::RwLock, time::Instant};

use self::{
    routes::common_types::DiscordApiError,
    types::{DApiVersion, Result},
};

pub mod routes;
mod types;
pub mod versions;

pub use self::types::{DApiDELETE, DApiError, DApiGET, DApiPATCH, DApiPOST, DApiPUT};

pub struct DApi<V: DApiVersion> {
    http: reqwest::Client,
    token: Option<String>,
    user_agent: String,
    api_base: String,
    api_ver: PhantomData<V>,
    ratelimits: RwLock<HashMap<String, Mutex<Option<Instant>>>>,
    global_limit: Mutex<Option<Instant>>,
}

impl<V: DApiVersion> DApi<V> {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: reqwest::ClientBuilder::new()
                .timeout(Duration::from_secs(5))
                .build()
                .map_err(DApiError::Instantiation)?,
            token: None,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Safari/537.36".to_owned(),
            api_base: format!("https://discord.com/api/{}", V::VER),
            api_ver: Default::default(),
            ratelimits: Default::default(),
            global_limit: Default::default(),
        })
    }

    pub fn set_token(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
    }

    pub fn set_user_agent(&mut self, user_agent: impl Into<String>) {
        self.user_agent = user_agent.into();
    }

    fn path_into_resource(path: &str) -> Option<&str> {
        let cap = regex!(r"(?:(?:channels)|(?:guilds)|(?:webhooks))/+(\d{4,21})").captures(path);
        return if let Some(c) = cap {
            c.get(1).map(|c| c.as_str())
        } else {
            None
        };
    }

    async fn request<R: DeserializeOwned, B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<R> {
        let resource = Self::path_into_resource(path);

        loop {
            {
                let retry_at = *self.global_limit.lock().unwrap();
                if let Some(retry_at) = retry_at {
                    tokio::time::sleep_until(retry_at).await;
                    *self.global_limit.lock().unwrap() = None;
                }
            }

            if let Some(r) = resource {
                let lock = self.ratelimits.read().await;
                let retry_at_lock = lock.get(r);
                if let Some(v) = retry_at_lock {
                    let retry_at = *v.lock().unwrap();
                    if let Some(retry_at) = retry_at {
                        drop(lock);
                        tokio::time::sleep_until(retry_at).await;
                        *self.ratelimits.read().await.get(r).unwrap().lock().unwrap() = None;
                    }
                } else {
                    drop(lock);
                    self.ratelimits
                        .write()
                        .await
                        .insert(r.into(), Mutex::new(None));
                }
            }

            let mut req = self.http.request(method.clone(), self.api_base.clone() + path)
                .header("User-Agent", &self.user_agent)
                .header("Content-Type", "application/json");

            if let Some(ref token) = self.token {
                req = req.header("Authorization", token);
            }

            if let Some(b) = body {
                req = req.json(b);
            }

            let resp = req.send().await.map_err(DApiError::Requesting)?;

            if resp.status().as_u16() == 429 {
                if let (Some(scope), Some(retry_after)) = (
                    resp.headers().get("x-ratelimit-scope"),
                    resp.headers().get("retry-after"),
                ) {
                    let scope = scope
                        .to_str()
                        .map_err(|e| DApiError::ParsingResponse(e.into()))?;
                    let retry_after = Instant::now()
                        + Duration::from_secs(
                            retry_after
                                .to_str()
                                .map_err(|e| DApiError::ParsingResponse(e.into()))?
                                .parse::<u64>()
                                .map_err(|e| DApiError::ParsingResponse(e.into()))?,
                        );
                    if scope == "user" || scope == "global" {
                        *self.global_limit.lock().unwrap() = Some(retry_after);
                    } else if let Some(r) = resource {
                        //shared
                        *self.ratelimits.read().await.get(r).unwrap().lock().unwrap() =
                            Some(retry_after);
                    } else {
                        return Err(DApiError::ParsingResponse(format!("Shared ratelimit encountered, however resource ID could not be parsed: {path}").into()));
                    }
                }
            } else if resp.status().as_u16() >= 400 {
                let err = resp.error_for_status_ref().unwrap_err();
                if let Ok(err) = resp.json().await {
                    return Err(DApiError::ApiError(err));
                }
                return Err(DApiError::ApiErrorWithoutBody(err));
            } else if resp.status().as_u16() == 204 {
                //no content responses, type R should be wrapped in an Option to produce a None value
                return serde_json::from_str("null")
                    .map_err(|e| DApiError::ParsingResponse(e.into()));
            } else {
                //success
                return resp
                    .json()
                    .await
                    .map_err(|e| DApiError::ParsingResponse(e.into()));
            }
        }
    }

    pub async fn get<T: DApiGET<V> + ?Sized>(&self, route: &T) -> Result<T::Response> {
        self.request(Method::GET, route.path(), None as Option<&()>)
            .await
    }

    pub async fn post<T: DApiPOST<V> + ?Sized>(
        &self,
        route: &T,
        body: &T::Body,
    ) -> Result<T::Response> {
        self.request(Method::POST, route.path(), Some(body)).await
    }

    pub async fn put<T: DApiPUT<V> + ?Sized>(
        &self,
        route: &T,
        body: &T::Body,
    ) -> Result<T::Response> {
        self.request(Method::PUT, route.path(), Some(body)).await
    }

    pub async fn patch<T: DApiPATCH<V> + ?Sized>(
        &self,
        route: &T,
        body: &T::Body,
    ) -> Result<T::Response> {
        self.request(Method::PATCH, route.path(), Some(body)).await
    }

    pub async fn delete<T: DApiDELETE<V> + ?Sized>(
        &self,
        route: &T,
        body: &T::Body,
    ) -> Result<T::Response> {
        self.request(Method::DELETE, route.path(), Some(body)).await
    }
}
