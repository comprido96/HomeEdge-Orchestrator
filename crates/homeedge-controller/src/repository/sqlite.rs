use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use homeedge_types::node::{NodeId, NodeRecord, NodeStatus};
use homeedge_types::service::{
    ServiceAssignment, ServiceDefinition, ServiceHealthReport, ServiceId, ServiceStatus,
};

use crate::repository::{
    AssignmentRepository, NodeRepository, ObservedStatusRepository, RepositoryError,
    ServiceRepository,
};


#[derive(Clone)]
pub struct SqliteNodeRepository {
    pool: SqlitePool,
}

impl SqliteNodeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NodeRepository for SqliteNodeRepository {
    async fn upsert(&self, node: &NodeRecord) -> Result<(), RepositoryError> {
        let now = Utc::now().to_rfc3339();
        let capabilities_json = serde_json::to_string(&node.capabilities)?;
        let last_heartbeat = node.last_heartbeat.map(|ts| ts.to_rfc3339());

        sqlx::query(
            r#"
            INSERT INTO nodes (
                node_id,
                status,
                last_heartbeat,
                capabilities_json,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(node_id) DO UPDATE SET
                status = excluded.status,
                last_heartbeat = excluded.last_heartbeat,
                capabilities_json = excluded.capabilities_json,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(node.id.to_string())
        .bind(encode_node_status(node.status))
        .bind(last_heartbeat)
        .bind(capabilities_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get(&self, node_id: NodeId) -> Result<Option<NodeRecord>, RepositoryError> {
        let maybe_row = sqlx::query(
            r#"
            SELECT
                node_id,
                status,
                last_heartbeat,
                capabilities_json
            FROM nodes
            WHERE node_id = ?1
            "#,
        )
        .bind(node_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match maybe_row {
            Some(row) => Ok(Some(decode_node_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<NodeRecord>, RepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT
                node_id,
                status,
                last_heartbeat,
                capabilities_json
            FROM nodes
            ORDER BY node_id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(decode_node_row).collect()
    }

    async fn set_heartbeat(
        &self,
        node_id: NodeId,
        timestamp: DateTime<Utc>,
        status: NodeStatus,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            r#"
            UPDATE nodes
            SET
                last_heartbeat = ?2,
                status = ?3,
                updated_at = ?4
            WHERE node_id = ?1
            "#,
        )
        .bind(node_id.to_string())
        .bind(timestamp.to_rfc3339())
        .bind(encode_node_status(status))
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound("node"));
        }

        Ok(())
    }

    async fn set_status(
        &self,
        node_id: NodeId,
        status: NodeStatus,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            r#"
            UPDATE nodes
            SET
                status = ?2,
                updated_at = ?3
            WHERE node_id = ?1
            "#,
        )
        .bind(node_id.to_string())
        .bind(encode_node_status(status))
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound("node"));
        }

        Ok(())
    }
}


#[derive(Clone)]
pub struct SqliteServiceRepository {
    pool: SqlitePool,
}

impl SqliteServiceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServiceRepository for SqliteServiceRepository {
    async fn insert(&self, service: &ServiceDefinition) -> Result<(), RepositoryError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO services (
                service_id,
                name,
                version,
                selector,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(service.id.to_string())
        .bind(&service.name)
        .bind(&service.version)
        .bind(&service.selector)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) => {
                if db_err.is_unique_violation() {
                    return Err(RepositoryError::Conflict(format!(
                        "service '{}' version '{}' already exists",
                        service.name, service.version
                    )));
                }

                Err(RepositoryError::Storage(db_err.to_string()))
            }
            Err(err) => Err(RepositoryError::from(err)),
        }
    }

    async fn update(&self, service: &ServiceDefinition) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            r#"
            UPDATE services
            SET
                name = ?2,
                version = ?3,
                selector = ?4,
                updated_at = ?5
            WHERE service_id = ?1
            "#,
        )
        .bind(service.id.to_string())
        .bind(&service.name)
        .bind(&service.version)
        .bind(&service.selector)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await;

        match result {
            Ok(result) => {
                if result.rows_affected() == 0 {
                    return Err(RepositoryError::NotFound("service"));
                }
                Ok(())
            }
            Err(sqlx::Error::Database(db_err)) => {
                if db_err.is_unique_violation() {
                    return Err(RepositoryError::Conflict(format!(
                        "service '{}' version '{}' already exists",
                        service.name, service.version
                    )));
                }

                Err(RepositoryError::Storage(db_err.to_string()))
            }
            Err(err) => Err(RepositoryError::from(err)),
        }
    }

    async fn get(
        &self,
        service_id: ServiceId,
    ) -> Result<Option<ServiceDefinition>, RepositoryError> {
        let maybe_row = sqlx::query(
            r#"
            SELECT
                service_id,
                name,
                version,
                selector
            FROM services
            WHERE service_id = ?1
            "#,
        )
        .bind(service_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match maybe_row {
            Some(row) => Ok(Some(decode_service_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<ServiceDefinition>, RepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT
                service_id,
                name,
                version,
                selector
            FROM services
            ORDER BY name, version, service_id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(decode_service_row).collect()
    }

    async fn delete(&self, service_id: ServiceId) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            r#"
            DELETE FROM services
            WHERE service_id = ?1
            "#,
        )
        .bind(service_id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound("service"));
        }

        Ok(())
    }
}


#[derive(Clone)]
pub struct SqliteAssignmentRepository {
    pool: SqlitePool,
}

impl SqliteAssignmentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AssignmentRepository for SqliteAssignmentRepository {
    async fn assign(
        &self,
        service_id: ServiceId,
        node_id: NodeId,
    ) -> Result<(), RepositoryError> {
        let now = Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO assignments (
                service_id,
                node_id,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(service_id) DO UPDATE SET
                node_id = excluded.node_id,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(service_id.to_string())
        .bind(node_id.to_string())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) => {
                Err(RepositoryError::Storage(db_err.to_string()))
            }
            Err(err) => Err(RepositoryError::from(err)),
        }
    }

    async fn unassign(
        &self,
        service_id: ServiceId,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            r#"
            DELETE FROM assignments
            WHERE service_id = ?1
            "#,
        )
        .bind(service_id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound("assignment"));
        }

        Ok(())
    }

    async fn for_node(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<ServiceAssignment>, RepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT
                service_id,
                node_id
            FROM assignments
            WHERE node_id = ?1
            ORDER BY service_id
            "#,
        )
        .bind(node_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let service_id: String = row.try_get("service_id")?;
                let node_id: String = row.try_get("node_id")?;

                Ok(ServiceAssignment {
                    service_id: parse_service_id(&service_id)?,
                    node_id: parse_node_id(&node_id)?,
                })
            })
            .collect()
    }

    async fn list(&self) -> Result<Vec<ServiceAssignment>, RepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT
                service_id,
                node_id
            FROM assignments
            ORDER BY node_id, service_id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let service_id: String = row.try_get("service_id")?;
                let node_id: String = row.try_get("node_id")?;

                Ok(ServiceAssignment {
                    service_id: parse_service_id(&service_id)?,
                    node_id: parse_node_id(&node_id)?,
                })
            })
            .collect()
    }

    async fn remove_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<ServiceId>, RepositoryError> {
        let assigned = self.for_node(node_id).await?;
        let service_ids: Vec<ServiceId> = assigned.iter().map(|a| a.service_id).collect();

        sqlx::query(
            r#"
            DELETE FROM assignments
            WHERE node_id = ?1
            "#,
        )
        .bind(node_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(service_ids)
    }
}


#[derive(Clone)]
pub struct SqliteObservedStatusRepository {
    pool: SqlitePool,
}

impl SqliteObservedStatusRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ObservedStatusRepository for SqliteObservedStatusRepository {
    async fn replace_for_node(
        &self,
        node_id: NodeId,
        statuses: &[ServiceHealthReport],
    ) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            DELETE FROM observed_service_status
            WHERE node_id = ?1
            "#,
        )
        .bind(node_id.to_string())
        .execute(&mut *tx)
        .await?;

        for report in statuses {
            sqlx::query(
                r#"
                INSERT INTO observed_service_status (
                    node_id,
                    service_id,
                    status,
                    updated_at
                )
                VALUES (?1, ?2, ?3, ?4)
                "#,
            )
            .bind(node_id.to_string())
            .bind(report.service_id.to_string())
            .bind(encode_service_status(report.status))
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    async fn list_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<ServiceHealthReport>, RepositoryError> {
        let rows = sqlx::query(
            r#"
            SELECT
                service_id,
                status
            FROM observed_service_status
            WHERE node_id = ?1
            ORDER BY service_id
            "#,
        )
        .bind(node_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let service_id: String = row.try_get("service_id")?;
                let status: String = row.try_get("status")?;

                Ok(ServiceHealthReport {
                    service_id: parse_service_id(&service_id)?,
                    status: decode_service_status(&status)?,
                })
            })
            .collect()
    }

    async fn clear_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            DELETE FROM observed_service_status
            WHERE node_id = ?1
            "#,
        )
        .bind(node_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}


fn parse_uuid(value: &str) -> Result<Uuid, RepositoryError> {
    Uuid::parse_str(value)
        .map_err(|e| RepositoryError::Storage(format!("invalid uuid '{value}': {e}")))
}

fn parse_node_id(value: &str) -> Result<NodeId, RepositoryError> {
    Ok(NodeId(parse_uuid(value)?))
}

fn parse_service_id(value: &str) -> Result<ServiceId, RepositoryError> {
    Ok(ServiceId(parse_uuid(value)?))
}

fn encode_node_status(status: NodeStatus) -> &'static str {
    match status {
        NodeStatus::Registering => "registering",
        NodeStatus::Healthy => "healthy",
        NodeStatus::Offline => "offline",
    }
}

fn decode_node_status(value: &str) -> Result<NodeStatus, RepositoryError> {
    match value {
        "registering" => Ok(NodeStatus::Registering),
        "healthy" => Ok(NodeStatus::Healthy),
        "offline" => Ok(NodeStatus::Offline),
        other => Err(RepositoryError::Storage(format!(
            "invalid node status '{other}'"
        ))),
    }
}

fn encode_service_status(status: ServiceStatus) -> &'static str {
    match status {
        ServiceStatus::Assigned => "assigned",
        ServiceStatus::Starting => "starting",
        ServiceStatus::Running => "running",
        ServiceStatus::Failed => "failed",
    }
}

fn decode_service_status(value: &str) -> Result<ServiceStatus, RepositoryError> {
    match value {
        "assigned" => Ok(ServiceStatus::Assigned),
        "starting" => Ok(ServiceStatus::Starting),
        "running" => Ok(ServiceStatus::Running),
        "failed" => Ok(ServiceStatus::Failed),
        other => Err(RepositoryError::Storage(format!(
            "invalid service status '{other}'"
        ))),
    }
}

fn decode_node_row(row: &sqlx::sqlite::SqliteRow) -> Result<NodeRecord, RepositoryError> {
    let node_id: String = row.try_get("node_id")?;
    let status: String = row.try_get("status")?;
    let last_heartbeat: Option<String> = row.try_get("last_heartbeat")?;
    let capabilities_json: String = row.try_get("capabilities_json")?;

    let last_heartbeat = match last_heartbeat {
        Some(ts) => Some(
            DateTime::parse_from_rfc3339(&ts)
                .map_err(|e| RepositoryError::Storage(format!("invalid timestamp '{ts}': {e}")))?
                .with_timezone(&Utc),
        ),
        None => None,
    };

    let capabilities: Vec<String> = serde_json::from_str(&capabilities_json)?;

    Ok(NodeRecord {
        id: parse_node_id(&node_id)?,
        status: decode_node_status(&status)?,
        last_heartbeat,
        capabilities,
    })
}

fn decode_service_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<ServiceDefinition, RepositoryError> {
    let service_id: String = row.try_get("service_id")?;
    let name: String = row.try_get("name")?;
    let version: String = row.try_get("version")?;
    let selector: Option<String> = row.try_get("selector")?;

    Ok(ServiceDefinition {
        id: parse_service_id(&service_id)?,
        name,
        version,
        selector,
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, Utc};
    use sqlx::SqlitePool;
    use uuid::Uuid;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite should connect");

        let migration = include_str!("../../migrations/001_initial.sql");

        for statement in migration.split(';') {
            let sql: &str = statement.trim();
            if sql.is_empty() {
                continue;
            }

            sqlx::query(sql)
                .execute(&pool)
                .await
                .expect("migration statement should succeed");
        }

        pool
    }

    fn node_id(n: u128) -> NodeId {
        NodeId(Uuid::from_u128(n))
    }

    fn service_id(n: u128) -> ServiceId {
        ServiceId(Uuid::from_u128(n))
    }

    fn sample_node(id: NodeId, status: NodeStatus) -> NodeRecord {
        NodeRecord {
            id,
            status,
            last_heartbeat: None,
            capabilities: vec!["docker".into(), "mqtt".into()],
        }
    }

    fn sample_service(id: ServiceId, name: &str, version: &str) -> ServiceDefinition {
        ServiceDefinition {
            id,
            name: name.to_string(),
            version: version.to_string(),
            selector: None,
        }
    }

    // --------------------------------------------------------------------- //
    // Node repository
    // --------------------------------------------------------------------- //

    #[tokio::test]
    async fn node_upsert_then_get_round_trips() {
        let pool = test_pool().await;
        let repo = SqliteNodeRepository::new(pool);

        let id = node_id(1);
        let node = sample_node(id, NodeStatus::Registering);

        repo.upsert(&node).await.unwrap();

        let stored = repo.get(id).await.unwrap().expect("node should exist");

        assert_eq!(stored.id, node.id);
        assert_eq!(stored.status, NodeStatus::Registering);
        assert_eq!(stored.last_heartbeat, None);
        assert_eq!(stored.capabilities, vec!["docker", "mqtt"]);
    }

    #[tokio::test]
    async fn node_upsert_updates_existing_node() {
        let pool = test_pool().await;
        let repo = SqliteNodeRepository::new(pool);

        let id = node_id(2);

        repo.upsert(&sample_node(id, NodeStatus::Registering))
            .await
            .unwrap();

        let updated = NodeRecord {
            id,
            status: NodeStatus::Healthy,
            last_heartbeat: Some(Utc::now()),
            capabilities: vec!["zigbee".into()],
        };

        repo.upsert(&updated).await.unwrap();

        let stored = repo.get(id).await.unwrap().unwrap();
        assert_eq!(stored.status, NodeStatus::Healthy);
        assert_eq!(stored.capabilities, vec!["zigbee"]);
        assert_eq!(stored.last_heartbeat, updated.last_heartbeat);
    }

    #[tokio::test]
    async fn node_list_returns_sorted_nodes() {
        let pool = test_pool().await;
        let repo = SqliteNodeRepository::new(pool);

        let id_b = node_id(20);
        let id_a = node_id(10);

        repo.upsert(&sample_node(id_b, NodeStatus::Healthy)).await.unwrap();
        repo.upsert(&sample_node(id_a, NodeStatus::Registering)).await.unwrap();

        let nodes = repo.list().await.unwrap();

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id, id_a);
        assert_eq!(nodes[1].id, id_b);
    }

    #[tokio::test]
    async fn node_set_heartbeat_updates_timestamp_and_status() {
        let pool = test_pool().await;
        let repo = SqliteNodeRepository::new(pool);

        let id = node_id(3);
        repo.upsert(&sample_node(id, NodeStatus::Registering)).await.unwrap();

        let ts = Utc::now();
        repo.set_heartbeat(id, ts, NodeStatus::Healthy).await.unwrap();

        let stored = repo.get(id).await.unwrap().unwrap();
        assert_eq!(stored.status, NodeStatus::Healthy);
        assert_eq!(stored.last_heartbeat, Some(ts));
    }

    #[tokio::test]
    async fn node_set_status_updates_status_only() {
        let pool = test_pool().await;
        let repo = SqliteNodeRepository::new(pool);

        let id = node_id(4);
        let mut node = sample_node(id, NodeStatus::Healthy);
        let ts = Utc::now() - ChronoDuration::seconds(30);
        node.last_heartbeat = Some(ts);

        repo.upsert(&node).await.unwrap();
        repo.set_status(id, NodeStatus::Offline).await.unwrap();

        let stored = repo.get(id).await.unwrap().unwrap();
        assert_eq!(stored.status, NodeStatus::Offline);
        assert_eq!(stored.last_heartbeat, Some(ts));
    }

    // --------------------------------------------------------------------- //
    // Service repository
    // --------------------------------------------------------------------- //

    #[tokio::test]
    async fn service_insert_then_get_round_trips() {
        let pool = test_pool().await;
        let repo = SqliteServiceRepository::new(pool);

        let id = service_id(100);
        let service = sample_service(id, "lighting", "v1");

        repo.insert(&service).await.unwrap();

        let stored = repo.get(id).await.unwrap().expect("service should exist");

        assert_eq!(stored.id, service.id);
        assert_eq!(stored.name, "lighting");
        assert_eq!(stored.version, "v1");
        assert_eq!(stored.selector, None);
    }

    #[tokio::test]
    async fn service_list_returns_sorted_services() {
        let pool = test_pool().await;
        let repo = SqliteServiceRepository::new(pool);

        repo.insert(&sample_service(service_id(2), "zigbee", "v2")).await.unwrap();
        repo.insert(&sample_service(service_id(1), "lighting", "v1")).await.unwrap();

        let services = repo.list().await.unwrap();

        assert_eq!(services.len(), 2);
        assert_eq!(services[0].name, "lighting");
        assert_eq!(services[1].name, "zigbee");
    }

    #[tokio::test]
    async fn service_update_persists_changes() {
        let pool = test_pool().await;
        let repo = SqliteServiceRepository::new(pool);

        let id = service_id(101);
        let mut service = sample_service(id, "lighting", "v1");
        repo.insert(&service).await.unwrap();

        service.name = "lighting-pro".into();
        service.version = "v2".into();
        service.selector = Some("room=living".into());

        repo.update(&service).await.unwrap();

        let stored = repo.get(id).await.unwrap().unwrap();
        assert_eq!(stored.name, "lighting-pro");
        assert_eq!(stored.version, "v2");
        assert_eq!(stored.selector.as_deref(), Some("room=living"));
    }

    #[tokio::test]
    async fn service_delete_removes_row() {
        let pool = test_pool().await;
        let repo = SqliteServiceRepository::new(pool);

        let id = service_id(102);
        repo.insert(&sample_service(id, "mqtt", "v1")).await.unwrap();

        repo.delete(id).await.unwrap();

        let stored = repo.get(id).await.unwrap();
        assert!(stored.is_none());
    }

    // --------------------------------------------------------------------- //
    // Assignment repository
    // --------------------------------------------------------------------- //

    #[tokio::test]
    async fn assignment_assign_then_for_node_returns_assignment() {
        let pool = test_pool().await;
        let node_repo = SqliteNodeRepository::new(pool.clone());
        let service_repo = SqliteServiceRepository::new(pool.clone());
        let repo = SqliteAssignmentRepository::new(pool);

        let nid = node_id(200);
        let sid = service_id(300);

        node_repo.upsert(&sample_node(nid, NodeStatus::Healthy)).await.unwrap();
        service_repo.insert(&sample_service(sid, "lighting", "v1")).await.unwrap();

        repo.assign(sid, nid).await.unwrap();

        let assignments = repo.for_node(nid).await.unwrap();
        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].service_id, sid);
        assert_eq!(assignments[0].node_id, nid);
    }

    #[tokio::test]
    async fn assignment_reassign_moves_service_to_new_node() {
        let pool = test_pool().await;
        let node_repo = SqliteNodeRepository::new(pool.clone());
        let service_repo = SqliteServiceRepository::new(pool.clone());
        let repo = SqliteAssignmentRepository::new(pool);

        let node_a = node_id(201);
        let node_b = node_id(202);
        let sid = service_id(301);

        node_repo.upsert(&sample_node(node_a, NodeStatus::Healthy)).await.unwrap();
        node_repo.upsert(&sample_node(node_b, NodeStatus::Healthy)).await.unwrap();
        service_repo.insert(&sample_service(sid, "lighting", "v1")).await.unwrap();

        repo.assign(sid, node_a).await.unwrap();
        repo.assign(sid, node_b).await.unwrap();

        let a_assignments = repo.for_node(node_a).await.unwrap();
        let b_assignments = repo.for_node(node_b).await.unwrap();

        assert!(a_assignments.is_empty());
        assert_eq!(b_assignments.len(), 1);
        assert_eq!(b_assignments[0].service_id, sid);
        assert_eq!(b_assignments[0].node_id, node_b);
    }

    #[tokio::test]
    async fn assignment_unassign_removes_assignment() {
        let pool = test_pool().await;
        let node_repo = SqliteNodeRepository::new(pool.clone());
        let service_repo = SqliteServiceRepository::new(pool.clone());
        let repo = SqliteAssignmentRepository::new(pool);

        let nid = node_id(203);
        let sid = service_id(302);

        node_repo.upsert(&sample_node(nid, NodeStatus::Healthy)).await.unwrap();
        service_repo.insert(&sample_service(sid, "mqtt", "v1")).await.unwrap();

        repo.assign(sid, nid).await.unwrap();
        repo.unassign(sid).await.unwrap();

        let assignments = repo.for_node(nid).await.unwrap();
        assert!(assignments.is_empty());
    }

    #[tokio::test]
    async fn assignment_remove_for_node_returns_removed_services() {
        let pool = test_pool().await;
        let node_repo = SqliteNodeRepository::new(pool.clone());
        let service_repo = SqliteServiceRepository::new(pool.clone());
        let repo = SqliteAssignmentRepository::new(pool);

        let nid = node_id(204);
        let sid1 = service_id(303);
        let sid2 = service_id(304);

        node_repo.upsert(&sample_node(nid, NodeStatus::Healthy)).await.unwrap();
        service_repo.insert(&sample_service(sid1, "lighting", "v1")).await.unwrap();
        service_repo.insert(&sample_service(sid2, "zigbee", "v1")).await.unwrap();

        repo.assign(sid1, nid).await.unwrap();
        repo.assign(sid2, nid).await.unwrap();

        let removed = repo.remove_for_node(nid).await.unwrap();

        assert_eq!(removed.len(), 2);
        assert!(removed.contains(&sid1));
        assert!(removed.contains(&sid2));
        assert!(repo.for_node(nid).await.unwrap().is_empty());
    }

    // --------------------------------------------------------------------- //
    // Observed status repository
    // --------------------------------------------------------------------- //

    #[tokio::test]
    async fn observed_replace_then_list_round_trips() {
        let pool = test_pool().await;
        let node_repo = SqliteNodeRepository::new(pool.clone());
        let service_repo = SqliteServiceRepository::new(pool.clone());
        let repo = SqliteObservedStatusRepository::new(pool);

        let nid = node_id(400);
        let sid1 = service_id(500);
        let sid2 = service_id(501);

        node_repo.upsert(&sample_node(nid, NodeStatus::Healthy)).await.unwrap();
        service_repo.insert(&sample_service(sid1, "lighting", "v1")).await.unwrap();
        service_repo.insert(&sample_service(sid2, "mqtt", "v1")).await.unwrap();

        let reports = vec![
            ServiceHealthReport {
                service_id: sid1,
                status: ServiceStatus::Running,
            },
            ServiceHealthReport {
                service_id: sid2,
                status: ServiceStatus::Failed,
            },
        ];

        repo.replace_for_node(nid, &reports).await.unwrap();

        let stored = repo.list_for_node(nid).await.unwrap();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].service_id, sid1);
        assert_eq!(stored[0].status, ServiceStatus::Running);
        assert_eq!(stored[1].service_id, sid2);
        assert_eq!(stored[1].status, ServiceStatus::Failed);
    }

    #[tokio::test]
    async fn observed_replace_for_node_replaces_previous_snapshot() {
        let pool = test_pool().await;
        let node_repo = SqliteNodeRepository::new(pool.clone());
        let service_repo = SqliteServiceRepository::new(pool.clone());
        let repo = SqliteObservedStatusRepository::new(pool);

        let nid = node_id(401);
        let sid1 = service_id(502);
        let sid2 = service_id(503);

        node_repo.upsert(&sample_node(nid, NodeStatus::Healthy)).await.unwrap();
        service_repo.insert(&sample_service(sid1, "lighting", "v1")).await.unwrap();
        service_repo.insert(&sample_service(sid2, "mqtt", "v1")).await.unwrap();

        repo.replace_for_node(
            nid,
            &[ServiceHealthReport {
                service_id: sid1,
                status: ServiceStatus::Running,
            }],
        )
        .await
        .unwrap();

        repo.replace_for_node(
            nid,
            &[ServiceHealthReport {
                service_id: sid2,
                status: ServiceStatus::Starting,
            }],
        )
        .await
        .unwrap();

        let stored = repo.list_for_node(nid).await.unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].service_id, sid2);
        assert_eq!(stored[0].status, ServiceStatus::Starting);
    }

    #[tokio::test]
    async fn observed_clear_for_node_removes_snapshot() {
        let pool = test_pool().await;
        let node_repo = SqliteNodeRepository::new(pool.clone());
        let service_repo = SqliteServiceRepository::new(pool.clone());
        let repo = SqliteObservedStatusRepository::new(pool);

        let nid = node_id(402);
        let sid = service_id(504);

        node_repo.upsert(&sample_node(nid, NodeStatus::Healthy)).await.unwrap();
        service_repo.insert(&sample_service(sid, "zigbee", "v1")).await.unwrap();

        repo.replace_for_node(
            nid,
            &[ServiceHealthReport {
                service_id: sid,
                status: ServiceStatus::Running,
            }],
        )
        .await
        .unwrap();

        repo.clear_for_node(nid).await.unwrap();

        let stored = repo.list_for_node(nid).await.unwrap();
        assert!(stored.is_empty());
    }
}
