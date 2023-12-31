use std::collections::HashMap;

use anyhow::{anyhow, Context};
use reqwest::StatusCode;

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
    pub async fn retrieve_token_list(&self) -> anyhow::Result<HashMap<String, TokenListEntry>> {
        let request = self
            .c
            .get("https://token.jup.ag/all")
            .header("Content-Type", "application/json")
            .build()?;
        Ok(self
            .c
            .execute(request)
            .await?
            .json::<Vec<TokenListEntry>>()
            .await?
            .into_iter()
            .map(|t| (t.address.clone(), t))
            .collect())
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
        let res = self
            .c
            .execute(request)
            .await
            .with_context(|| "failed to execute quote lookup")?;
        if res.status().ne(&StatusCode::OK) {
            return Err(anyhow!("quote lookup failed {}", res.text().await?));
        }
        Ok(res
            .json()
            .await
            .with_context(|| "failed to decode quote lookup response")?)
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
        let res = self
            .c
            .execute(request)
            .await
            .with_context(|| "failed to execute new_swap")?;
        if res.status().ne(&StatusCode::OK) {
            return Err(anyhow!("new_swap failed {}", res.text().await?));
        }
        Ok(res
            .json()
            .await
            .with_context(|| "failed to deserialize new_swap response")?)
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
        let res = self
            .c
            .execute(request)
            .await
            .with_context(|| "failed to execute price query")?;
        if res.status().ne(&StatusCode::OK) {
            return Err(anyhow!("price lookup failed {}", res.text().await?));
        }
        Ok(res
            .json()
            .await
            .with_context(|| "faled to deserialize price query")?)
    }
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenListEntry {
    pub address: String,
    pub chain_id: i64,
    pub decimals: i64,
    pub name: String,
    pub symbol: String,
    #[serde(rename = "logoURI")]
    pub logo_uri: Option<String>,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_token_list() {
        let client = Client::new().unwrap();
        let tokens = client.retrieve_token_list().await.unwrap();
        println!(
            "{:#?}",
            tokens
                .get("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
                .unwrap()
        );
    }
    #[tokio::test]
    async fn test_jlp_usdc_swap() {
        let client = Client::new().unwrap();

        let response = client
            .new_quote(
                "27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4",
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                1000000,
                &[RequestOption::SlippageBps(100)],
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
