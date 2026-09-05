use rest::RestClient;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};

use super::common::{AiToolError, TavilySearchArgs, TavilySearchOutput, TavilySearchResultOutput};

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

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let query = args.query.trim();
        if query.is_empty() {
            return Err(AiToolError("query must not be empty".to_string()));
        }

        let request = TavilyRequest {
            query: query.to_string(),
            search_depth: args.search_depth.as_deref().filter(|depth| !depth.trim().is_empty()).map(str::to_string),
            topic: args.topic.as_deref().filter(|topic| !topic.trim().is_empty()).map(str::to_string),
            max_results: args.max_results,
            include_domains: args.include_domains,
            exclude_domains: args.exclude_domains,
            time_range: args.time_range.as_deref().filter(|range| !range.trim().is_empty()).map(str::to_string),
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
