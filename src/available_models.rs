use openrouter_rs::{api::models::Model, config::OpenRouterConfig, OpenRouterClient};

/// Fetch available models from OpenRouter API
pub async fn fetch_models_from_api(
    client: &OpenRouterClient,
) -> Result<Vec<Model>, Box<dyn std::error::Error>> {
    // OpenRouter models endpoint
    let models = client.list_models().await?;
    Ok(models)
}

/// Check if a model is free based on pricing
pub fn is_free_model(model: &Model) -> bool {
    // Check if both prompt and completion pricing are "0"
    model.pricing.prompt == "0" && model.pricing.completion == "0"
}

/// Get curated list of recommended models (fallback when API is unavailable)
pub fn list_available_models() -> Vec<&'static str> {
    vec![
        // FREE MODELS (🆓)
        "qwen/qwen3-coder:free",            // Qwen Coder - Free coding model
        "google/gemini-2.0-flash-exp:free", // Google Gemini 2.0 Flash
        "google/gemini-flash-1.5:free",     // Google Gemini 1.5 Flash
        "meta-llama/llama-3.1-8b-instruct:free", // Meta Llama 3.1 8B
        "meta-llama/llama-3.2-3b-instruct:free", // Meta Llama 3.2 3B
        "microsoft/phi-3-mini-128k-instruct:free", // Microsoft Phi-3 Mini
        "nousresearch/hermes-3-llama-3.1-405b:free", // Hermes 3 405B
        // PAID MODELS - Anthropic Claude (recommended for ReAct)
        "anthropic/claude-sonnet-4",
        "anthropic/claude-3.5-sonnet",
        "anthropic/claude-3-opus",
        // PAID MODELS - OpenAI
        "openai/gpt-4-turbo",
        "openai/gpt-4o",
        "openai/gpt-3.5-turbo",
        // PAID MODELS - DeepSeek (good for reasoning)
        "deepseek/deepseek-r1",
        "deepseek/deepseek-chat",
        // PAID MODELS - Meta
        "meta-llama/llama-3.3-70b-instruct",
    ]
}

/// Get models from OpenRouter config presets
/// This uses OpenRouter's built-in model categorization
pub fn get_preset_models(preset: &str) -> Vec<String> {
    let config = OpenRouterConfig::default();

    match preset {
        "programming" | "reasoning" | "free" => {
            // OpenRouter config has built-in presets
            config.get_resolved_models()
        }
        _ => vec![],
    }
}

/// List all available presets
pub fn list_presets() -> Vec<&'static str> {
    vec![
        "programming", // Code generation and development
        "reasoning",   // Advanced problem-solving models
        "free",        // Free-tier models for experimentation
    ]
}
