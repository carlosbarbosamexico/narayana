use std::time::Duration;
use tokio::time::sleep;

// Integration tests for the HTTP server
// These would require starting the server, so they're marked as integration tests

#[tokio::test]
#[ignore] // Ignore by default - requires server to be running
async fn test_health_endpoint() {
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8080/health")
        .timeout(Duration::from_secs(5))
        .send()
        .await;
    
    if let Ok(resp) = response {
        assert!(resp.status().is_success());
        let json: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(json["status"], "healthy");
    }
}

#[tokio::test]
#[ignore]
async fn test_create_table_endpoint() {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/api/v1/tables")
        .json(&serde_json::json!({
            "table_name": "test_table",
            "schema": {
                "fields": [
                    {
                        "name": "id",
                        "data_type": "Int64",
                        "nullable": false
                    }
                ]
            }
        }))
        .timeout(Duration::from_secs(5))
        .send()
        .await;
    
    if let Ok(resp) = response {
        assert!(resp.status().is_success());
        let json: serde_json::Value = resp.json().await.unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["table_id"].as_u64().is_some());
    }
}

#[tokio::test]
#[ignore]
async fn test_stats_endpoint() {
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8080/api/v1/stats")
        .timeout(Duration::from_secs(5))
        .send()
        .await;
    
    if let Ok(resp) = response {
        assert!(resp.status().is_success());
        let json: serde_json::Value = resp.json().await.unwrap();
        assert!(json["total_queries"].is_number());
        assert!(json["avg_duration_ms"].is_number());
    }
}

