// This module is only compiled when running tests
#![cfg(any(test, feature = "testing"))]

use crate::providers::{Provider, RateLimitInfo, RateLimitsConfig};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default, Debug)]
pub struct MockProvider {
    pub calls: Arc<Mutex<Vec<String>>>,
    pub responses: Arc<Mutex<HashMap<String, RateLimitInfo>>>,
    pub default_response: Arc<Mutex<Option<RateLimitInfo>>>,
}

impl MockProvider {
    #[must_use]
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Box<dyn Provider> {
        Box::new(Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            responses: Arc::new(Mutex::new(HashMap::new())),
            default_response: Arc::new(Mutex::new(Some(RateLimitInfo {
                requests_used: 0,
                tokens_used: 0,
                input_tokens_used: 0,
            }))),
        })
    }

    /// Set the response that will be returned by this mock provider
    ///
    /// # Panics
    ///
    /// Will panic if the mutex is poisoned
    pub fn set_response(&self, info: RateLimitInfo) {
        *self.default_response.lock().unwrap() = Some(info);
    }

    /// Get a list of all API calls made to this mock provider
    ///
    /// # Panics
    ///
    /// Will panic if the mutex is poisoned
    ///
    /// # Returns
    ///
    /// Returns a vector of strings representing the API calls made
    #[must_use]
    pub fn get_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl Provider for MockProvider {
    fn get_rate_limits(&self) -> Result<RateLimitInfo> {
        self.calls
            .lock()
            .unwrap()
            .push("get_rate_limits".to_string());
        Ok(self
            .default_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(RateLimitInfo {
                requests_used: 0,
                tokens_used: 0,
                input_tokens_used: 0,
            }))
    }

    fn get_rate_limits_config(&self) -> Result<RateLimitsConfig> {
        self.calls
            .lock()
            .unwrap()
            .push("get_rate_limits_config".to_string());
        Ok(RateLimitsConfig {
            requests_per_minute: Some(100),
            tokens_per_minute: Some(1000),
            input_tokens_per_minute: Some(500),
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
