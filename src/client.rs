use anyhow::Context;

use crate::{
    price_types::{format_price_url, PriceResponse},
    quote_types::{format_quote_url, QuoteResponse, RequestOption},
    swap_types::{SwapRequest, SwapResponse, SWAP_BASE},
};

pub struct Client {
    c: reqwest::Client,
}

impl Client {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            c: reqwest::ClientBuilder::new()
                .brotli(true)
                .gzip(true)
                .deflate(true)
                .build()?,
        })
    }
    pub async fn new_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        request_options: &[RequestOption<'_>],
    ) -> anyhow::Result<QuoteResponse> {
        let request_url = format_quote_url(input_mint, output_mint, amount, request_options);
        let request = self
            .c
            .get(request_url)
            .header("Content-Type", "application/json")
            .build()?;
        Ok(self
            .c
            .execute(request)
            .await
            .with_context(|| "failed to execute quote lookup")?
            .json()
            .await
            .with_context(|| "failed to decode response body")?)
    }
    pub async fn new_swap(
        &self,
        quote: QuoteResponse,
        user_public_key: &str,
        wrap_unwrap_sol: bool,
    ) -> anyhow::Result<SwapResponse> {
        let req_body = SwapRequest {
            user_public_key: user_public_key.to_string(),
            wrap_and_unwrap_sol: wrap_unwrap_sol,
            quote_response: quote,
            ..Default::default()
        };

        let request = self
            .c
            .post(SWAP_BASE)
            .header("Content-Type", "application/json")
            .json(&req_body)
            .build()?;
        Ok(self.c.execute(request).await?.json().await?)
    }
    pub async fn price_query(
        &self,
        input_mint: &str,
        output_mint: &str,
    ) -> anyhow::Result<PriceResponse> {
        let request = self
            .c
            .get(format_price_url(input_mint, output_mint))
            .header("Content-Type", "application/json")
            .build()?;
        Ok(self
            .c
            .execute(request)
            .await
            .with_context(|| "failed to execute price query")?
            .json()
            .await
            .with_context(|| "faled to deserialize price query")?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_jlp_usdc_swap() {
        let client = Client::new().unwrap();

        let response = client
            .new_quote(
                "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4",
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                1000000,
                &[],
            )
            .await
            .unwrap();

        let response = client
            .new_swap(
                response,
                "5WVCN6gmtCMt61W47aaQ9ByA3Lvfn85ALtTD2VQhLrdx",
                true,
            )
            .await
            .unwrap();
        let response = serde_json::to_string_pretty(&response).unwrap();
        //println!("{}", response);

        let price_response = client
            .price_query(
                "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4",
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            )
            .await
            .unwrap();
        println!("{:#?}", price_response);
    }
}
