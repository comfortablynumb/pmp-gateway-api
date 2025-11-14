use crate::config::{MongoOperation, MongodbClientConfig};
use anyhow::Result;
use mongodb::{bson::Document, Client, Collection, Database};
use serde_json::Value;
use tracing::{debug, info};

/// MongoDB client
#[derive(Debug, Clone)]
pub struct MongodbClient {
    database: Database,
}

impl MongodbClient {
    /// Create a new MongoDB client
    pub async fn new(config: MongodbClientConfig) -> Result<Self> {
        info!("Creating MongoDB client for database: {}", config.database);

        let client = Client::with_uri_str(&config.connection_string).await?;
        let database = client.database(&config.database);

        Ok(Self { database })
    }

    /// Execute a MongoDB operation
    pub async fn execute_operation(
        &self,
        collection_name: &str,
        operation: &MongoOperation,
    ) -> Result<MongoResponse> {
        debug!(
            "Executing MongoDB operation on collection: {}",
            collection_name
        );

        let collection: Collection<Document> = self.database.collection(collection_name);

        match operation {
            MongoOperation::Find { filter, limit } => {
                let filter_doc: Document = serde_json::from_str(filter)?;

                let mut cursor = collection.find(filter_doc, None).await?;

                let mut documents = Vec::new();
                let limit_val = limit.unwrap_or(100);
                let mut count = 0;

                use futures::stream::StreamExt;
                while let Some(result) = cursor.next().await {
                    if count >= limit_val {
                        break;
                    }
                    let doc = result?;
                    let json: Value = serde_json::to_value(&doc)?;
                    documents.push(json);
                    count += 1;
                }

                let count = documents.len();
                Ok(MongoResponse {
                    documents,
                    count,
                    operation_type: "find".to_string(),
                })
            }

            MongoOperation::FindOne { filter } => {
                let filter_doc: Document = serde_json::from_str(filter)?;
                let result = collection.find_one(filter_doc, None).await?;

                let documents = if let Some(doc) = result {
                    vec![serde_json::to_value(&doc)?]
                } else {
                    vec![]
                };

                Ok(MongoResponse {
                    documents,
                    count: 1,
                    operation_type: "findOne".to_string(),
                })
            }

            MongoOperation::Insert { document } => {
                let doc: Document = serde_json::from_str(document)?;
                let result = collection.insert_one(doc, None).await?;

                Ok(MongoResponse {
                    documents: vec![serde_json::json!({
                        "inserted_id": result.inserted_id.to_string()
                    })],
                    count: 1,
                    operation_type: "insert".to_string(),
                })
            }

            MongoOperation::Update { filter, update } => {
                let filter_doc: Document = serde_json::from_str(filter)?;
                let update_doc: Document = serde_json::from_str(update)?;

                let result = collection.update_many(filter_doc, update_doc, None).await?;

                Ok(MongoResponse {
                    documents: vec![serde_json::json!({
                        "matched_count": result.matched_count,
                        "modified_count": result.modified_count
                    })],
                    count: result.modified_count as usize,
                    operation_type: "update".to_string(),
                })
            }

            MongoOperation::Delete { filter } => {
                let filter_doc: Document = serde_json::from_str(filter)?;
                let result = collection.delete_many(filter_doc, None).await?;

                Ok(MongoResponse {
                    documents: vec![serde_json::json!({
                        "deleted_count": result.deleted_count
                    })],
                    count: result.deleted_count as usize,
                    operation_type: "delete".to_string(),
                })
            }
        }
    }
}

/// MongoDB response structure
#[derive(Debug, Clone)]
pub struct MongoResponse {
    pub documents: Vec<Value>,
    pub count: usize,
    pub operation_type: String,
}
