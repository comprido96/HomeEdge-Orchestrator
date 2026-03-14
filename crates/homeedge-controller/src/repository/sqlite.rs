use homeedge_types::{NodeId, NodeRecord};

// TODO: add ServiceRepository and AssignmentRepository in post-demo scope
#[async_trait::async_trait]
pub trait NodeRepository {
    async fn get_node(&self, _id: NodeId) -> Option<NodeRecord>;

    async fn save_node(&self, _node: NodeRecord);

    async fn list_nodes(&self) -> Vec<NodeRecord>;
}

pub struct SqliteRepository;

impl SqliteRepository {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl NodeRepository for SqliteRepository {
    async fn get_node(&self, _id: NodeId) -> Option<NodeRecord> {
        todo!()
    }

    async fn save_node(&self, _node: NodeRecord) {
        todo!()
    }

    async fn list_nodes(&self) -> Vec<NodeRecord> {
        todo!()
    }
}
