//! Fixed-origin, bounded native transport; no error body is exposed.
use control_core::error::{Error, Result};
use oauth2::{AsyncHttpClient, HttpRequest, HttpResponse};
use std::{future::Future, pin::Pin};
#[derive(Clone)]
pub struct Http {
    pub(crate) client: reqwest::Client,
}
impl Http {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .https_only(true)
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|_| Error::new("google_client_unavailable", 503))?,
        })
    }
    pub async fn json(
        &self,
        url: &str,
        token: &str,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let mut header = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| Error::new("google_credential_unavailable", 503))?;
        header.set_sensitive(true);
        let response = self
            .client
            .get(url)
            .header(reqwest::header::AUTHORIZATION, header)
            .query(query)
            .send()
            .await
            .map_err(|_| Error::uncertain("google_request_failed", 502))?;
        let bytes = self.bytes(response, 262144).await?;
        serde_json::from_slice(&bytes).map_err(|_| Error::uncertain("google_response_invalid", 502))
    }
    pub async fn bytes(&self, mut response: reqwest::Response, max: usize) -> Result<Vec<u8>> {
        match response.status().as_u16() {
            200 => (),
            401 | 403 | 404 => return Err(Error::new("google_access_denied", 403)),
            429 => return Err(Error::new("google_rate_limited", 429)),
            _ => return Err(Error::uncertain("google_request_failed", 502)),
        }
        let mut data = vec![];
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Error::uncertain("google_response_invalid", 502))?
        {
            if data.len() + chunk.len() > max {
                return Err(Error::uncertain("google_result_too_large", 502));
            }
            data.extend(chunk);
        }
        Ok(data)
    }
}
impl<'c> AsyncHttpClient<'c> for Http {
    type Error = std::io::Error;
    type Future = Pin<
        Box<dyn Future<Output = std::result::Result<HttpResponse, Self::Error>> + Send + Sync + 'c>,
    >;
    fn call(&'c self, request: HttpRequest) -> Self::Future {
        Box::pin(async move {
            let err = || std::io::Error::other("bounded OAuth transport failed");
            let mut response = self
                .client
                .execute(request.try_into().map_err(|_| err())?)
                .await
                .map_err(|_| err())?;
            let mut builder = oauth2::http::Response::builder().status(response.status());
            for (key, value) in response.headers() {
                builder = builder.header(key, value);
            }
            let mut data = vec![];
            while let Some(chunk) = response.chunk().await.map_err(|_| err())? {
                if data.len() + chunk.len() > 16384 {
                    return Err(err());
                }
                data.extend(chunk);
            }
            builder.body(data).map_err(|_| err())
        })
    }
}
