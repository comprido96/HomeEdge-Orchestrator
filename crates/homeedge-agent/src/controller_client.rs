use chrono::Utc;
use homeedge_types::{AssignmentsResponse, ListServicesResponse, ServiceDefinition, ServiceId, ServiceStatus};
use reqwest::StatusCode;

use homeedge_types::api::{HeartbeatRequest, RegisterRequest};
use homeedge_types::node::NodeId;
use homeedge_types::service::{ServiceAssignment, ServiceHealthReport};

use crate::error::AgentError;

#[derive(Debug, Clone)]
pub struct HeartbeatPayload {
    pub service_statuses: Vec<(ServiceId, ServiceStatus)>,
}


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
        operation: &'static str,
        response: reqwest::Response,
    ) -> Result<reqwest::Response, AgentError> {
        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        let message = match response.text().await {
            Ok(body) if !body.is_empty() => body,
            Ok(_) => "<empty body>".to_string(),
            Err(err) => format!("<failed to read error body: {err}>"),
        };

        Err(AgentError::ControllerApi {
            operation,
            status,
            message,
        })
    }

    pub async fn register(&self) -> Result<(), AgentError> {
        let req = RegisterRequest {
            node_id: self.node_id,
            capabilities: self.capabilities.clone(),
        };

        let url = format!("{}/register", self.base_url);

        let response = self.http
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|source| AgentError::Http {
                operation: "register",
                source,
            })?;

        let response = Self::expect_success("register", response).await?;

        Ok(())
    }

    pub async fn heartbeat(&self, status: HeartbeatPayload) -> Result<(), AgentError> {
        let req = HeartbeatRequest {
            node_id: self.node_id,
            timestamp: Utc::now(),
            service_statuses: status.service_statuses
                .into_iter()
                .map(|(service_id, status)| ServiceHealthReport { service_id, status })
                .collect(),
        };

        let url = format!("{}/heartbeat", self.base_url);

        let response = self.http
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|source| AgentError::Http {
                operation: "heartbeat",
                source,
            })?;

        let response = Self::expect_success("heartbeat", response).await?;

        Ok(())
    }

    pub async fn get_assignments(&self) -> Result<Vec<ServiceAssignment>, AgentError> {
        let url = format!("{}/assignments/{}", self.base_url, self.node_id);
        let response = self.http
        .get(&url)
        .send()
        .await
        .map_err(|source| AgentError::Http {
            operation: "get_assignments",
            source,
        })?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AgentError::NodeNotRegistered(self.node_id));
        }

        let response = Self::expect_success("get_assignments", response).await?;
        let body: Vec<ServiceAssignment> = response.json().await.map_err(|source| AgentError::Http {
            operation: "get_assignments",
            source,
        })?;

        Ok(body)
    }
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub async fn list_services(&self) -> Result<Vec<ServiceDefinition>, AgentError> {
        let url = format!("{}/services", self.base_url);

        let response = self.http
        .get(&url)
        .send().await
        .map_err(|source| AgentError::Http {
            operation: "list_services",
            source,
        })?;
        let response = Self::expect_success("list_services", response).await?;
        let body: ListServicesResponse = response
        .json()
        .await
        .map_err(|source| AgentError::Http {
            operation: "list_services",
            source,
        })?;

        Ok(body.services)
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
            .heartbeat(HeartbeatPayload { service_statuses: vec![] })
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
