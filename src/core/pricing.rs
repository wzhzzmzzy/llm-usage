use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::RwLock;

use crate::core::error::PricingError;

const OPENROUTER_PRICING_URL: &str = "https://openrouter.ai/api/v1/models";
const PRICING_FETCH_TIMEOUT_SECONDS: u64 = 15;
const PRICING_CACHE_FILE: &str = "openrouter-pricing.json";

/// Pricing information for a single model
#[derive(Debug, Clone, Copy)]
pub struct Pricing {
    pub input: f64,
    pub output: f64,
    pub cache_create: f64,
    pub cache_read: f64,
    pub input_above_200k: Option<f64>,
    pub output_above_200k: Option<f64>,
    pub cache_create_above_200k: Option<f64>,
    pub cache_read_above_200k: Option<f64>,
}

/// Map of model name to pricing information
#[derive(Debug, Clone, Default)]
pub struct PricingMap {
    entries: HashMap<String, Pricing>,
    context_limits: HashMap<String, u64>,
}

/// OpenRouter API response structure
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    data: Vec<OpenRouterModel>,
}

/// OpenRouter model entry
#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    context_length: Option<u64>,
    pricing: OpenRouterPricing,
}

/// OpenRouter pricing fields (values are per-token USD)
#[derive(Debug, Deserialize)]
struct OpenRouterPricing {
    prompt: Option<String>,
    completion: Option<String>,
    input_cache_read: Option<String>,
    input_cache_write: Option<String>,
}

pub struct PricingCache {
    pricing: RwLock<PricingMap>,
    snapshot: std::sync::RwLock<PricingMap>,
    cache_dir: PathBuf,
}

impl PricingCache {
    pub fn new() -> Result<Self, PricingError> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("llm-usage");
        
        fs::create_dir_all(&cache_dir)
            .map_err(|e| PricingError::IoError(e.to_string()))?;
        
        Ok(Self {
            pricing: RwLock::new(PricingMap::default()),
            snapshot: std::sync::RwLock::new(PricingMap::default()),
            cache_dir,
        })
    }

    pub async fn ensure_loaded(&self) {
        {
            let pricing = self.pricing.read().await;
            if pricing.is_loaded() {
                return;
            }
        }

        if self.load_from_cache().await.is_ok() {
            let pricing = self.pricing.read().await;
            if pricing.is_loaded() {
                self.sync_snapshot(&pricing);
                return;
            }
        }

        let _ = self.fetch_and_cache().await;
        let pricing = self.pricing.read().await;
        self.sync_snapshot(&pricing);
    }

    pub fn get_snapshot_sync(&self) -> PricingMap {
        self.snapshot.read().unwrap().clone()
    }

    fn sync_snapshot(&self, pricing: &PricingMap) {
        let mut snap = self.snapshot.write().unwrap();
        *snap = pricing.clone();
    }

    pub async fn get_pricing(&self, model: &str) -> Option<Pricing> {
        let pricing = self.pricing.read().await;
        if let Some(p) = pricing.find(model) {
            return Some(p);
        }
        drop(pricing);
        
        if self.load_from_cache().await.is_ok() {
            let pricing = self.pricing.read().await;
            if let Some(p) = pricing.find(model) {
                return Some(p);
            }
        }
        
        if self.fetch_and_cache().await.is_ok() {
            let pricing = self.pricing.read().await;
            return pricing.find(model);
        }
        
        None
    }

    async fn load_from_cache(&self) -> Result<(), PricingError> {
        let cache_file = self.cache_dir.join(PRICING_CACHE_FILE);
        if !cache_file.exists() {
            return Err(PricingError::CacheNotFound);
        }
        
        let metadata = fs::metadata(&cache_file)
            .map_err(|e| PricingError::IoError(e.to_string()))?;
        let modified = metadata.modified()
            .map_err(|e| PricingError::IoError(e.to_string()))?;
        let elapsed = modified.elapsed()
            .map_err(|e| PricingError::IoError(e.to_string()))?;
        
        if elapsed > Duration::from_secs(24 * 60 * 60) {
            return Err(PricingError::CacheExpired);
        }
        
        let json = fs::read_to_string(&cache_file)
            .map_err(|e| PricingError::IoError(e.to_string()))?;
        
        let mut pricing = self.pricing.write().await;
        pricing.load_json(&json);
        
        Ok(())
    }

    async fn fetch_and_cache(&self) -> Result<(), PricingError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(PRICING_FETCH_TIMEOUT_SECONDS))
            .build()
            .map_err(|e| PricingError::FetchError(e.to_string()))?;
        
        let response = client
            .get(OPENROUTER_PRICING_URL)
            .send()
            .await
            .map_err(|e| PricingError::FetchError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(PricingError::FetchError(format!(
                "HTTP {}",
                response.status()
            )));
        }
        
        let json = response
            .text()
            .await
            .map_err(|e| PricingError::FetchError(e.to_string()))?;
        
        let cache_file = self.cache_dir.join(PRICING_CACHE_FILE);
        fs::write(&cache_file, &json)
            .map_err(|e| PricingError::IoError(e.to_string()))?;
        
        let mut pricing = self.pricing.write().await;
        pricing.load_json(&json);
        
        Ok(())
    }

    pub async fn refresh(&self) -> Result<(), PricingError> {
        self.fetch_and_cache().await
    }
}

impl PricingMap {
    /// Load pricing from JSON string
    pub fn load_json(&mut self, json: &str) -> usize {
        let Ok(response) = serde_json::from_str::<OpenRouterResponse>(json) else {
            return 0;
        };
        
        let mut loaded_count = 0;
        for model in response.data {
            if model.id.starts_with('~') {
                continue;
            }
            
            let Some(prompt_str) = model.pricing.prompt else {
                continue;
            };
            let Some(completion_str) = model.pricing.completion else {
                continue;
            };
            
            let input = prompt_str.parse::<f64>().unwrap_or(0.0);
            let output = completion_str.parse::<f64>().unwrap_or(0.0);
            
            if input <= 0.0 || output <= 0.0 {
                continue;
            }
            
            let cache_read = model.pricing.input_cache_read
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(input * 0.1);
            
            let cache_create = model.pricing.input_cache_write
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(input * 1.25);
            
            self.entries.insert(
                model.id.clone(),
                Pricing {
                    input,
                    output,
                    cache_create,
                    cache_read,
                    input_above_200k: None,
                    output_above_200k: None,
                    cache_create_above_200k: None,
                    cache_read_above_200k: None,
                },
            );
            
            if let Some(context_limit) = model.context_length {
                self.context_limits.insert(model.id, context_limit);
            }
            
            loaded_count += 1;
        }
        
        loaded_count
    }

    /// Find pricing for a model with fuzzy matching
    pub fn find(&self, model: &str) -> Option<Pricing> {
        self.entries.get(model).copied().or_else(|| {
            let normalized_model = normalized_pricing_key(model);
            self.entries
                .iter()
                .filter(|(candidate, _)| {
                    pricing_key_matches(candidate, model, normalized_model.as_ref())
                })
                .max_by(|(left, _), (right, _)| {
                    left.len().cmp(&right.len()).then_with(|| right.cmp(left))
                })
                .map(|(_, pricing)| *pricing)
        })
    }

    pub fn is_loaded(&self) -> bool {
        !self.entries.is_empty()
    }

    pub fn to_json(&self) -> serde_json::Value {
        let map: std::collections::HashMap<&str, serde_json::Value> = self.entries
            .iter()
            .map(|(model, pricing)| {
                (model.as_str(), serde_json::json!({
                    "input": pricing.input,
                    "output": pricing.output,
                    "cacheCreate": pricing.cache_create,
                    "cacheRead": pricing.cache_read,
                }))
            })
            .collect();
        serde_json::to_value(&map).unwrap_or_default()
    }

    /// Get context limit for a model
    pub fn context_limit(&self, model: &str) -> Option<u64> {
        self.context_limits.get(model).copied().or_else(|| {
            let normalized_model = normalized_pricing_key(model);
            self.context_limits
                .iter()
                .filter(|(candidate, _)| {
                    pricing_key_matches(candidate, model, normalized_model.as_ref())
                })
                .max_by(|(left, _), (right, _)| {
                    left.len().cmp(&right.len()).then_with(|| right.cmp(left))
                })
                .map(|(_, context_limit)| *context_limit)
        })
    }

    pub fn put_builtin_pricing(&mut self) {
        // Claude models
        self.entries.insert(
            "claude-opus-4".to_string(),
            Pricing {
                input: 15e-6,
                output: 75e-6,
                cache_create: 18.75e-6,
                cache_read: 1.5e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "claude-sonnet-4".to_string(),
            Pricing {
                input: 3e-6,
                output: 15e-6,
                cache_create: 3.75e-6,
                cache_read: 0.3e-6,
                input_above_200k: Some(6e-6),
                output_above_200k: Some(22.5e-6),
                cache_create_above_200k: Some(7.5e-6),
                cache_read_above_200k: Some(0.6e-6),
            },
        );
        self.entries.insert(
            "claude-3-5-haiku".to_string(),
            Pricing {
                input: 0.8e-6,
                output: 4e-6,
                cache_create: 1.0e-6,
                cache_read: 0.08e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "claude-3-opus".to_string(),
            Pricing {
                input: 15e-6,
                output: 75e-6,
                cache_create: 18.75e-6,
                cache_read: 1.5e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "claude-3-sonnet".to_string(),
            Pricing {
                input: 3e-6,
                output: 15e-6,
                cache_create: 3.75e-6,
                cache_read: 0.3e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "claude-3-haiku".to_string(),
            Pricing {
                input: 0.25e-6,
                output: 1.25e-6,
                cache_create: 0.3e-6,
                cache_read: 0.03e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );

        // GPT models
        self.entries.insert(
            "gpt-5".to_string(),
            Pricing {
                input: 1.25e-6,
                output: 10e-6,
                cache_create: 1.25e-6,
                cache_read: 0.125e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "gpt-5.1".to_string(),
            Pricing {
                input: 1.25e-6,
                output: 10e-6,
                cache_create: 1.25e-6,
                cache_read: 0.125e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "gpt-5.2".to_string(),
            Pricing {
                input: 1.75e-6,
                output: 14e-6,
                cache_create: 1.75e-6,
                cache_read: 0.175e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "gpt-5.4".to_string(),
            Pricing {
                input: 2.5e-6,
                output: 15e-6,
                cache_create: 2.5e-6,
                cache_read: 0.25e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "gpt-5.4-mini".to_string(),
            Pricing {
                input: 0.75e-6,
                output: 4.5e-6,
                cache_create: 0.75e-6,
                cache_read: 0.075e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "gpt-5.4-mini-fast".to_string(),
            Pricing {
                input: 0.75e-6,
                output: 4.5e-6,
                cache_create: 0.75e-6,
                cache_read: 0.075e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "mimo-v2.5-pro".to_string(),
            Pricing {
                input: 0.44e-6,
                output: 0.88e-6,
                cache_create: 0.55e-6,
                cache_read: 0.0036e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        self.entries.insert(
            "gpt-5.4-nano".to_string(),
            Pricing {
                input: 0.2e-6,
                output: 1.25e-6,
                cache_create: 0.2e-6,
                cache_read: 0.02e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );

        // Context limits
        self.context_limits.insert("gpt-5.5".to_string(), 1_050_000);
        self.context_limits.insert("gpt-5.4".to_string(), 1_050_000);
        for model in ["claude-opus-4", "claude-sonnet-4"] {
            self.context_limits.insert(model.to_string(), 1_000_000);
        }
        for model in [
            "claude-3-5-haiku",
            "claude-3-5-haiku-20241022",
            "claude-3-opus",
            "claude-3-sonnet",
            "claude-3-haiku",
        ] {
            self.context_limits.insert(model.to_string(), 200_000);
        }
    }
}

/// Matches pricing keys across provider/model aliases
fn pricing_key_matches(candidate: &str, model: &str, normalized_model: &str) -> bool {
    if contains_pricing_key(model, candidate) || contains_pricing_key(candidate, model) {
        return true;
    }
    let normalized_candidate = normalized_pricing_key(candidate);
    contains_pricing_key(normalized_model, normalized_candidate.as_ref())
        || contains_pricing_key(normalized_candidate.as_ref(), normalized_model)
}

/// Finds a key only when the surrounding bytes are non-alphanumeric boundaries
fn contains_pricing_key(value: &str, key: &str) -> bool {
    value.match_indices(key).any(|(index, _)| {
        let before = index
            .checked_sub(1)
            .and_then(|before| value.as_bytes().get(before))
            .copied();
        let after = value.as_bytes().get(index + key.len()).copied();
        before.is_none_or(is_pricing_key_boundary) && after.is_none_or(is_pricing_key_boundary)
    })
}

/// Treats punctuation separators as boundaries
fn is_pricing_key_boundary(byte: u8) -> bool {
    !byte.is_ascii_alphanumeric()
}

/// Normalizes known model separator variants
fn normalized_pricing_key(value: &str) -> std::borrow::Cow<'_, str> {
    if value.contains(['.', '@']) {
        std::borrow::Cow::Owned(value.replace(['.', '@'], "-"))
    } else {
        std::borrow::Cow::Borrowed(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_map_fuzzy_matching() {
        let mut pricing = PricingMap::default();
        pricing.entries.insert(
            "claude-sonnet-4".to_string(),
            Pricing {
                input: 3e-6,
                output: 15e-6,
                cache_create: 3.75e-6,
                cache_read: 0.3e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );
        pricing.entries.insert(
            "claude-sonnet-4-20250514".to_string(),
            Pricing {
                input: 3e-6,
                output: 15e-6,
                cache_create: 3.75e-6,
                cache_read: 0.3e-6,
                input_above_200k: None,
                output_above_200k: None,
                cache_create_above_200k: None,
                cache_read_above_200k: None,
            },
        );

        assert!(pricing.find("claude-sonnet-4-20250514").is_some());
        assert!(pricing.find("claude-sonnet-4").is_some());
        assert!(pricing.find("anthropic.claude-sonnet-4-20250514-v1:0").is_some());
    }
}
