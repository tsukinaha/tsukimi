use futures::{
    FutureExt,
    future::BoxFuture,
};

use super::*;

pub struct Route<C, K> {
    client: C,
    kind: K,
}

impl<C, K> Route<C, K>
where
    C: ClientPrelude,
{
    pub fn new(client: &C, kind: K) -> Self {
        Self {
            client: client.clone(),
            kind,
        }
    }
}

impl<C, Re: Request> std::fmt::Display for Route<C, Re> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str(&self.kind.path())
    }
}

impl<C, Re> IntoFuture for Route<C, Re>
where
    C: ClientPrelude,
    Re: Request,
{
    type Output = Result<Re::Response>;

    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            let path = self.kind.path();
            let mut request = self
                .client
                .client()
                .request(Re::METHOD, format!("{}{}", C::BASE_URI, path));

            if let Some(headers) = self.client.headers(&path) {
                request = request.headers(headers)
            }

            if let Some(body) = self.kind.body() {
                request = request.json(&body);
            }

            if let Some(params) = self.kind.params() {
                request = request.query(&params);
            }

            let response = request.send().await.map_err(Error::HttpError)?;

            let data = response
                .json::<Re::Response>()
                .await
                .map_err(Error::HttpError)?;

            Ok(data)
        }
        .boxed()
    }
}
