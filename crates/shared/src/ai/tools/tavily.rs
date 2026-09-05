use rest::RestClient;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};

use super::common::{
    AiToolError, TavilyExtractArgs, TavilyExtractFailedResultOutput, TavilyExtractOutput,
    TavilyExtractResultOutput, TavilySearchArgs, TavilySearchOutput,
    TavilySearchResultOutput,
};

#[derive(Clone)]
pub struct TavilySearchTool {
    api_key: String,
}

impl TavilySearchTool {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct TavilyRequest {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_depth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time_range: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TavilyApiResponse {
    results: Vec<TavilyApiResult>,
}

#[derive(Debug, Deserialize)]
struct TavilyApiResult {
    title: String,
    url: String,
    content: String,
    #[serde(default)]
    score: f64,
    #[serde(default)]
    published_date: Option<String>,
}

impl Tool for TavilySearchTool {
    const NAME: &'static str = "tavily_search";

    type Error = AiToolError;
    type Args = TavilySearchArgs;
    type Output = TavilySearchOutput;

    fn description(&self) -> String {
        "Search the web for current information and return concise results with titles, URLs, summaries, and relevance scores.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for Tavily web search."
                },
                "search_depth": {
                    "type": "string",
                    "enum": ["basic", "advanced"],
                    "description": "Depth of the search."
                },
                "topic": {
                    "type": "string",
                    "enum": ["general", "news", "finance"],
                    "description": "Topic category for the query."
                },
                "max_results": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 20,
                    "description": "Maximum number of result items to return."
                },
                "include_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Restricted domains to include in the results."
                },
                "exclude_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Domains to exclude from the results."
                },
                "time_range": {
                    "type": "string",
                    "enum": ["day", "week", "month", "year"],
                    "description": "Time window for search results."
                }
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }

    async fn call(
        &self,
        _context: &mut rig::tool::ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let query = args.query.trim();
        if query.is_empty() {
            return Err(AiToolError("query must not be empty".to_string()));
        }

        let request = TavilyRequest {
            query: query.to_string(),
            search_depth: args
                .search_depth
                .as_deref()
                .filter(|depth| !depth.trim().is_empty())
                .map(str::to_string),
            topic: args
                .topic
                .as_deref()
                .filter(|topic| !topic.trim().is_empty())
                .map(str::to_string),
            max_results: args.max_results,
            include_domains: args.include_domains,
            exclude_domains: args.exclude_domains,
            time_range: args
                .time_range
                .as_deref()
                .filter(|range| !range.trim().is_empty())
                .map(str::to_string),
        };

        let client = RestClient::builder()
            .base_url("https://api.tavily.com")
            .bearer_auth(self.api_key.clone())
            .build()?;

        let response: TavilyApiResponse = client
            .post("/search")
            .json(&request)?
            .json_response()
            .await?;

        let results = response
            .results
            .into_iter()
            .map(|entry| TavilySearchResultOutput {
                title: entry.title,
                url: entry.url,
                content: entry.content,
                score: entry.score,
                published_date: entry.published_date,
            })
            .collect();

        Ok(TavilySearchOutput {
            query: query.to_string(),
            results,
        })
    }
}

#[derive(Clone)]
pub struct TavilyExtractTool {
    api_key: String,
}

impl TavilyExtractTool {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct TavilyExtractRequest {
    urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extract_depth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TavilyExtractApiResponse {
    results: Vec<TavilyExtractApiResult>,
    #[serde(default)]
    failed_results: Vec<TavilyExtractApiFailedResult>,
}

#[derive(Debug, Deserialize)]
struct TavilyExtractApiResult {
    url: String,
    raw_content: String,
    #[serde(default)]
    images: Option<Vec<String>>,
    #[serde(default)]
    favicon: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TavilyExtractApiFailedResult {
    url: String,
    error: String,
}

impl Tool for TavilyExtractTool {
    const NAME: &'static str = "tavily_extract";

    type Error = AiToolError;
    type Args = TavilyExtractArgs;
    type Output = TavilyExtractOutput;

    fn description(&self) -> String {
        "Read and extract the main text content from one or more URLs using Tavily. Use this when you need the actual article or page contents from a search result URL.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "urls": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "One or more URLs to extract content from."
                },
                "query": {
                    "type": "string",
                    "description": "Optional query to help Tavily focus extraction on the relevant part of each page."
                }
            },
            "required": ["urls"],
            "additionalProperties": false
        })
    }

    async fn call(
        &self,
        _context: &mut rig::tool::ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let urls = args
            .urls
            .into_iter()
            .map(|url| url.trim().to_string())
            .filter(|url| !url.is_empty())
            .collect::<Vec<_>>();
        if urls.is_empty() {
            return Err(AiToolError(
                "urls must contain at least one non-empty URL".to_string(),
            ));
        }

        let request = TavilyExtractRequest {
            urls: urls.clone(),
            query: args
                .query
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string),
            extract_depth: Some("basic".to_string()),
            format: Some("markdown".to_string()),
        };

        let client = RestClient::builder()
            .base_url("https://api.tavily.com")
            .bearer_auth(self.api_key.clone())
            .build()?;

        let response: TavilyExtractApiResponse = client
            .post("/extract")
            .json(&request)?
            .json_response()
            .await?;

        let results = response
            .results
            .into_iter()
            .map(|entry| TavilyExtractResultOutput {
                url: entry.url,
                raw_content: entry.raw_content,
                images: entry.images,
                favicon: entry.favicon,
            })
            .collect();

        let failed_results = response
            .failed_results
            .into_iter()
            .map(|entry| TavilyExtractFailedResultOutput {
                url: entry.url,
                error: entry.error,
            })
            .collect();

        Ok(TavilyExtractOutput {
            results,
            failed_results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tavily_extract_args_and_response_round_trip() {
        let args = TavilyExtractArgs {
            urls: vec!["https://example.com".to_string()],
            query: Some("rust async".to_string()),
        };

        let request = serde_json::json!({
            "urls": args.urls,
            "query": args.query,
        });

        assert_eq!(request["urls"][0], "https://example.com");
        assert_eq!(request["query"], "rust async");

        let response = TavilyExtractOutput {
            results: vec![TavilyExtractResultOutput {
                url: "https://example.com".to_string(),
                raw_content: "content".to_string(),
                images: Some(vec!["https://example.com/a.png".to_string()]),
                favicon: Some("https://example.com/favicon.ico".to_string()),
            }],
            failed_results: vec![],
        };

        assert_eq!(response.results.len(), 1);
        assert!(response.failed_results.is_empty());
    }
}
