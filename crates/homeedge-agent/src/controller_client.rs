use chrono::Utc;
use reqwest::StatusCode;

use homeedge_types::api::{AssignmentsResponse, HeartbeatRequest, RegisterRequest};
use homeedge_types::node::NodeId;
use homeedge_types::service::ServiceId;

use crate::error::AgentError;

#[derive(Debug, Clone, Default)]
pub struct HeartbeatPayload;

#[derive(Clone)]
pub struct ControllerClient {
    http: reqwest::Client,
    base_url: String,
    node_id: NodeId,
    capabilities: Vec<String>,
}

impl ControllerClient {
    pub fn new(
        base_url: impl Into<String>,
        node_id: NodeId,
        capabilities: Vec<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            node_id,
            capabilities,
        }
    }

    async fn expect_success(
        response: reqwest::Response,
    ) -> Result<reqwest::Response, AgentError> {
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status();
            let message = response.text().await.unwrap_or_default();
            Err(AgentError::ControllerApi { status, message })
        }
    }

    pub async fn register(&self) -> Result<(), AgentError> {
        let req = RegisterRequest {
            node_id: self.node_id,
            capabilities: self.capabilities.clone(),
        };

        let url = format!("{}/register", self.base_url);
        let response = self.http.post(&url).json(&req).send().await?;
        Self::expect_success(response).await?;
        Ok(())
    }

    pub async fn heartbeat(&self, _status: HeartbeatPayload) -> Result<(), AgentError> {
        let req = HeartbeatRequest {
            node_id: self.node_id,
            timestamp: Utc::now(),
            service_statuses: Vec::new(),
        };

        let url = format!("{}/heartbeat", self.base_url);
        let response = self.http.post(&url).json(&req).send().await?;
        Self::expect_success(response).await?;
        Ok(())
    }

    pub async fn get_assignments(&self) -> Result<Vec<ServiceId>, AgentError> {
        let url = format!("{}/assignments/{}", self.base_url, self.node_id);

        let response = self.http.get(&url).send().await?;

        // Check for 404 before expect_success because expect_success
        // consumes the response body for error messages.
        if response.status() == StatusCode::NOT_FOUND {
            return Err(AgentError::NodeNotRegistered(self.node_id));
        }

        let response = Self::expect_success(response).await?;
        let body: AssignmentsResponse = response.json().await?;

        Ok(body.service_ids)
    }

    pub fn node_id(&self) -> homeedge_types::node::NodeId {
        self.node_id
    }
}


#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use homeedge_types::api::AssignmentsResponse;
    use homeedge_types::node::NodeId;
    use homeedge_types::service::ServiceId;

    use super::{ControllerClient, HeartbeatPayload};

    fn make_client(base_url: &str, node_id: NodeId) -> ControllerClient {
        ControllerClient::new(base_url, node_id, vec!["docker".into()])
    }

    // ------------------------------------------------------------------ //
    // register()
    // ------------------------------------------------------------------ //

    #[tokio::test]
    async fn register_sends_correct_json_to_register_endpoint() {
        let server = MockServer::start().await;
        let node_id = NodeId(Uuid::new_v4());

        let expected_body = serde_json::json!({
            "node_id": node_id.0,
            "capabilities": ["docker"]
        });

        Mock::given(method("POST"))
            .and(path("/register"))
            .and(header("content-type", "application/json"))
            .and(body_json(&expected_body))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let client = make_client(&server.uri(), node_id);
        client.register().await.expect("register should succeed");
    }

    // ------------------------------------------------------------------ //
    // heartbeat()
    // ------------------------------------------------------------------ //

    #[tokio::test]
    async fn heartbeat_sends_correct_node_id_and_recent_timestamp() {
        let server = MockServer::start().await;
        let node_id = NodeId(Uuid::new_v4());

        let before = Utc::now();

        Mock::given(method("POST"))
            .and(path("/heartbeat"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let client = make_client(&server.uri(), node_id);
        client
            .heartbeat(HeartbeatPayload)
            .await
            .expect("heartbeat should succeed");

        let after = Utc::now();

        // Verify the request body that wiremock actually received.
        let received = &server.received_requests().await.unwrap()[0];
        let body: serde_json::Value =
            serde_json::from_slice(&received.body).expect("body should be valid JSON");

        assert_eq!(
            body["node_id"].as_str().unwrap(),
            node_id.0.to_string(),
            "node_id in heartbeat body must match agent identity"
        );

        let timestamp_str = body["timestamp"].as_str().unwrap();
        let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_str)
            .expect("timestamp should be RFC3339")
            .with_timezone(&Utc);

        assert!(
            timestamp >= before && timestamp <= after,
            "timestamp should be within the test window"
        );
    }

    // ------------------------------------------------------------------ //
    // get_assignments()
    // ------------------------------------------------------------------ //

    #[tokio::test]
    async fn get_assignments_parses_successful_response() {
        let server = MockServer::start().await;
        let node_id = NodeId(Uuid::new_v4());
        let service_id = ServiceId(Uuid::new_v4());

        let response_body = AssignmentsResponse {
            node_id,
            service_ids: vec![service_id],
        };

        Mock::given(method("GET"))
            .and(path(format!("/assignments/{}", node_id)))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(&response_body),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = make_client(&server.uri(), node_id);
        let assignments = client
            .get_assignments()
            .await
            .expect("get_assignments should succeed");

        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0], service_id);
    }

    #[tokio::test]
    async fn get_assignments_maps_404_to_node_not_registered() {
        let server = MockServer::start().await;
        let node_id = NodeId(Uuid::new_v4());

        Mock::given(method("GET"))
            .and(path(format!("/assignments/{}", node_id)))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;

        let client = make_client(&server.uri(), node_id);
        let err = client
            .get_assignments()
            .await
            .expect_err("should return an error on 404");

        assert!(
            matches!(err, crate::error::AgentError::NodeNotRegistered(id) if id == node_id),
            "404 should map to NodeNotRegistered, got: {err:?}"
        );
    }
}
