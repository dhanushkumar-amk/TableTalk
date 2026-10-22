use tabletalk_backend::orchestrator::{
    bedrock::{AwsBedrockClient, BedrockClient},
    types::{BedrockToolSpec, ChatMessage},
};

#[tokio::test]
async fn test_real_bedrock_call() {
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "ap-south-1".to_string());
    let model_id = std::env::var("BEDROCK_MODEL_ID")
        .unwrap_or_else(|_| "apac.amazon.nova-lite-v1:0".to_string());

    println!(
        "Testing real AWS Bedrock call with region={} and model_id={}",
        region, model_id
    );

    let client = match AwsBedrockClient::from_env(&region).await {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to create AWS Bedrock client: {:?}", e);
            return;
        }
    };

    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: "What are the hours for TableTalk?".to_string(),
    }];

    let tools: Vec<BedrockToolSpec> = vec![];

    let result = client
        .converse(
            &model_id,
            "You are a helpful restaurant assistant. Keep replies brief.",
            &messages,
            &tools,
            200,
        )
        .await;

    match result {
        Ok(output) => {
            println!("REAL BEDROCK CALL SUCCEEDED!");
            if let Some(msg) = output.message {
                println!("Output: {}", msg.content);
            }
            println!("Token Usage: {:?}", output.token_usage);
        }
        Err(err) => {
            println!("REAL BEDROCK CALL RESULT (Error): {:?}", err);
        }
    }
}

#[tokio::test]
async fn test_real_chat_orchestrator_fallback_on_bedrock_unauthorized() {
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;
    use tabletalk_backend::{
        availability::FixedClock,
        db::{init_test_pool, seed_database},
        orchestrator::{ChatRequest, OrchestratorService, SessionManager},
        Config, TableTalkServer,
    };

    let pool = init_test_pool().await.unwrap();
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 21, 4, 0, 0).unwrap();
    seed_database(&pool, Some(fixed_now)).await.unwrap();

    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "ap-south-1".to_string());
    let client = Arc::new(AwsBedrockClient::from_env(&region).await.unwrap());

    let config = Config {
        aws_region: region,
        bedrock_model_id: "apac.amazon.nova-lite-v1:0".to_string(),
        chat_max_tokens: 500,
        chat_session_token_budget: 10000,
        chat_daily_token_budget: 50000,
        chat_rate_limit_per_minute: 60,
        chat_max_message_len: 1000,
        ..Config::default()
    };

    let session_manager = Arc::new(SessionManager::new(
        config.chat_session_token_budget,
        config.chat_daily_token_budget,
    ));

    let server = TableTalkServer::with_db(pool.clone());
    let clock = Arc::new(FixedClock(fixed_now));

    let service =
        OrchestratorService::new(client, session_manager, pool.clone(), server, clock, config);

    // Call chat endpoint with a menu query
    let req = ChatRequest {
        session_id: Some("live-test-session-1".to_string()),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "What vegan dishes do you have on the menu?".to_string(),
        }],
    };

    let resp = service.process_chat(req).await;
    println!("Chat response role: {}", resp.message.role);
    println!("Chat response text: {}", resp.message.content);
    println!("Fallback mode: {}", resp.fallback_mode);
    println!(
        "Structured contents count: {}",
        resp.structured_contents.len()
    );

    assert!(
        resp.fallback_mode,
        "Fallback mode should engage when Bedrock encounters Operation not allowed"
    );
    assert!(
        !resp.message.content.is_empty(),
        "Fallback response should provide helpful message"
    );
}
