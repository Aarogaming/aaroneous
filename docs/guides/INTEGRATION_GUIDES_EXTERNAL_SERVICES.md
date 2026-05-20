# Aaroneous Federation: Integration Guides for External Services

## Overview

Complete integration guides for connecting Aaroneous Federation with external services, APIs, and third-party platforms.

---

## Table of Contents

1. [Vector Databases](#vector-databases)
2. [Large Language Models (LLMs)](#large-language-models)
3. [Message Queues](#message-queues)
4. [Observability Services](#observability-services)
5. [Storage Services](#storage-services)
6. [Databases](#databases)
7. [Authentication Services](#authentication-services)
8. [Webhooks & Event Systems](#webhooks--event-systems)

---

## Vector Databases

### Pinecone Integration

For semantic search and embeddings storage.

```rust
use pinecone_db::{PineconeClient, IndexMetadata};
use aaroneous_sdk::*;

pub struct VectorStoreSpecialist {
    pinecone_client: Arc<Mutex<PineconeClient>>,
}

impl VectorStoreSpecialist {
    pub async fn new(api_key: &str, index_name: &str) -> Result<Self> {
        let client = PineconeClient::new(api_key, index_name).await?;
        Ok(Self {
            pinecone_client: Arc::new(Mutex::new(client)),
        })
    }

    pub async fn store_embedding(&self, id: &str, embedding: Vec<f32>, metadata: serde_json::Value) -> Result<()> {
        let client = self.pinecone_client.lock().unwrap();
        client.upsert(
            id,
            embedding,
            Some(metadata),
        ).await?;
        Ok(())
    }

    pub async fn semantic_search(&self, query_embedding: Vec<f32>, top_k: usize) -> Result<Vec<String>> {
        let client = self.pinecone_client.lock().unwrap();
        let results = client.query(query_embedding, top_k, None).await?;
        Ok(results.iter().map(|r| r.id.clone()).collect())
    }
}

#[async_trait]
impl Specialist for VectorStoreSpecialist {
    fn id(&self) -> SpecialistId {
        SpecialistId::from("vector-store")
    }

    fn name(&self) -> &str {
        "Vector Store Specialist"
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "semantic_search".to_string(),
            "similarity_matching".to_string(),
            "embedding_storage".to_string(),
        ]
    }

    async fn propose(&self, context: &Context) -> Result<Proposal> {
        let query_text = context.metadata
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or(SpecialistError::MissingParameter("query".to_string()))?;

        // Convert query to embedding (you'd use an embedding model)
        let embedding = self.text_to_embedding(query_text).await?;

        // Search Pinecone
        let matches = self.semantic_search(embedding, 5).await?;

        Ok(Proposal {
            proposal_id: generate_id(),
            specialist_id: self.id(),
            solution: ProposalSolution {
                solution_type: "semantic_search_results".to_string(),
                description: format!("Found {} semantic matches", matches.len()),
                parameters: serde_json::json!({
                    "matches": matches,
                    "count": matches.len(),
                }).as_object().unwrap().clone(),
                reasoning: "Semantic search using vector embeddings".to_string(),
            },
            confidence: 0.92,
            estimated_cost: Cost {
                compute_ms: 50,
                memory_mb: 100,
                storage_mb: 0,
                network_mb: 5,
            },
            dependencies: vec![],
            alternatives: vec![],
            metadata: Default::default(),
        })
    }

    async fn execute(&self, proposal: &Proposal) -> Result<ExecutionResult> {
        Ok(ExecutionResult {
            execution_id: generate_id(),
            specialist_id: self.id(),
            status: ExecutionStatus::Success,
            output: proposal.solution.parameters.clone(),
            metrics: Default::default(),
        })
    }

    async fn delegate(&self, task: Task) -> Result<DelegatedResult> {
        Ok(DelegatedResult {
            task_id: task.task_id,
            specialist_id: self.id(),
            success: true,
            output: serde_json::json!({}),
        })
    }

    async fn negotiate(&self, conflict: &Conflict) -> Result<Resolution> {
        let winner = conflict.proposals.first()
            .ok_or(SpecialistError::NoProposalsToNegotiate)?;

        Ok(Resolution {
            resolution_id: generate_id(),
            winning_proposal_id: winner.proposal_id.clone(),
            reasoning: "First proposal selected".to_string(),
            agreed_by: vec![self.id()],
        })
    }

    async fn learn(&self, feedback: &Feedback) -> Result<()> {
        // Improve future searches based on feedback
        Ok(())
    }

    fn serialize_state(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    async fn deserialize_state(&mut self, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus {
            healthy: true,
            uptime_secs: 0,
            last_proposal_ms: 50,
            error_rate: 0.0,
        })
    }
}
```

### Weaviate Integration

```rust
use weaviate_rust::client::Client;

pub struct WeaviateSpecialist {
    client: Arc<Client>,
}

impl WeaviateSpecialist {
    pub async fn new(url: &str) -> Result<Self> {
        let client = Client::new(url).await?;
        Ok(Self {
            client: Arc::new(client),
        })
    }

    pub async fn create_class(&self, schema: &str) -> Result<()> {
        self.client.create_class(schema).await?;
        Ok(())
    }

    pub async fn add_object(&self, class_name: &str, object: serde_json::Value) -> Result<String> {
        let id = self.client.add_object(class_name, object).await?;
        Ok(id)
    }

    pub async fn query(&self, query: &str) -> Result<serde_json::Value> {
        let results = self.client.query(query).await?;
        Ok(results)
    }
}
```

---

## Large Language Models (LLMs)

### OpenAI Integration

```rust
use openai_api::Client as OpenAIClient;

pub struct LLMSpecialist {
    openai_client: Arc<Mutex<OpenAIClient>>,
    model: String,
}

impl LLMSpecialist {
    pub fn new(api_key: &str, model: &str) -> Self {
        let client = OpenAIClient::new(api_key.to_string());
        Self {
            openai_client: Arc::new(Mutex::new(client)),
            model: model.to_string(),
        }
    }

    pub async fn complete(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        let client = self.openai_client.lock().unwrap();
        
        let response = client.create_completion(
            &self.model,
            prompt,
            max_tokens,
            0.7,  // temperature
        ).await?;

        Ok(response.choices[0].text.clone())
    }

    pub async fn chat(&self, messages: Vec<(String, String)>) -> Result<String> {
        let client = self.openai_client.lock().unwrap();
        
        let response = client.create_chat_completion(
            &self.model,
            &messages,
            0.7,
        ).await?;

        Ok(response.choices[0].message.content.clone())
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let client = self.openai_client.lock().unwrap();
        
        let embedding = client.create_embedding(
            "text-embedding-3-small",
            text,
        ).await?;

        Ok(embedding.data[0].embedding.clone())
    }
}

#[async_trait]
impl Specialist for LLMSpecialist {
    fn id(&self) -> SpecialistId {
        SpecialistId::from("llm-specialist")
    }

    fn name(&self) -> &str {
        "LLM Specialist"
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "text_generation".to_string(),
            "question_answering".to_string(),
            "summarization".to_string(),
            "embeddings".to_string(),
        ]
    }

    async fn propose(&self, context: &Context) -> Result<Proposal> {
        let prompt = context.metadata
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or(SpecialistError::MissingParameter("prompt".to_string()))?;

        let response = self.complete(prompt, 500).await?;

        Ok(Proposal {
            proposal_id: generate_id(),
            specialist_id: self.id(),
            solution: ProposalSolution {
                solution_type: "llm_response".to_string(),
                description: "Generated response using GPT-4".to_string(),
                parameters: serde_json::json!({
                    "response": response,
                    "model": "gpt-4",
                }).as_object().unwrap().clone(),
                reasoning: "LLM-based text generation".to_string(),
            },
            confidence: 0.85,
            estimated_cost: Cost {
                compute_ms: 2000,  // LLM calls are slower
                memory_mb: 512,
                storage_mb: 0,
                network_mb: 10,
            },
            dependencies: vec![],
            alternatives: vec![],
            metadata: Default::default(),
        })
    }

    // ... implement other trait methods ...
}
```

### Anthropic Claude Integration

```rust
use anthropic_rs::client::Client as AnthropicClient;

pub struct ClaudeSpecialist {
    client: Arc<Mutex<AnthropicClient>>,
}

impl ClaudeSpecialist {
    pub fn new(api_key: &str) -> Self {
        let client = AnthropicClient::new(api_key.to_string());
        Self {
            client: Arc::new(Mutex::new(client)),
        }
    }

    pub async fn message(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        let client = self.client.lock().unwrap();
        
        let response = client.messages()
            .model("claude-3-opus-20240229")
            .max_tokens(max_tokens)
            .messages(vec![
                anthropic_rs::Message::user(prompt),
            ])
            .create()
            .await?;

        Ok(response.content[0].text.clone())
    }
}
```

### Ollama (Local LLM) Integration

```rust
pub struct OllamaSpecialist {
    base_url: String,
    model: String,
}

impl OllamaSpecialist {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }

    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        
        let response = client.post(format!("{}/api/generate", self.base_url))
            .json(&serde_json::json!({
                "model": &self.model,
                "prompt": prompt,
                "stream": false,
            }))
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;
        Ok(body["response"].as_str().unwrap_or("").to_string())
    }
}
```

---

## Message Queues

### RabbitMQ Integration

```rust
use lapin::{Channel, Connection, ConnectionProperties};
use tokio::io::AsyncWriteExt;

pub struct MessageQueueSpecialist {
    connection: Arc<Mutex<Connection>>,
    channel: Arc<Mutex<Channel>>,
}

impl MessageQueueSpecialist {
    pub async fn new(url: &str) -> Result<Self> {
        let connection = Connection::connect(
            url,
            ConnectionProperties::default(),
        ).await?;

        let channel = connection.create_channel().await?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            channel: Arc::new(Mutex::new(channel)),
        })
    }

    pub async fn declare_queue(&self, queue_name: &str) -> Result<()> {
        let channel = self.channel.lock().unwrap();
        
        channel.queue_declare(
            queue_name,
            Default::default(),
            Default::default(),
        ).await?;

        Ok(())
    }

    pub async fn publish_message(&self, queue: &str, message: &str) -> Result<()> {
        let channel = self.channel.lock().unwrap();
        
        channel.basic_publish(
            "",
            queue,
            Default::default(),
            message.as_bytes(),
            Default::default(),
        ).await?;

        Ok(())
    }

    pub async fn consume_messages(&self, queue: &str) -> Result<()> {
        let channel = self.channel.lock().unwrap();
        
        let consumer = channel.basic_consume(
            queue,
            "consumer",
            Default::default(),
            Default::default(),
        ).await?;

        // Process messages
        Ok(())
    }
}
```

### Kafka Integration

```rust
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::consumer::{StreamConsumer, Consumer};

pub struct KafkaSpecialist {
    producer: Arc<FutureProducer>,
    consumer: Arc<StreamConsumer>,
}

impl KafkaSpecialist {
    pub async fn new(brokers: &str) -> Result<Self> {
        let producer: FutureProducer = rdkafka::client_config::ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .create()?;

        let consumer: StreamConsumer = rdkafka::client_config::ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", "aaroneous-consumer")
            .create()?;

        Ok(Self {
            producer: Arc::new(producer),
            consumer: Arc::new(consumer),
        })
    }

    pub async fn send_message(&self, topic: &str, key: &str, value: &str) -> Result<()> {
        let record = FutureRecord::to(topic)
            .key(key)
            .payload(value);

        self.producer.send_result(record)
            .await
            .map_err(|e| e.into())
            .and_then(|r| r.map_err(|e| e.into()))?;

        Ok(())
    }
}
```

---

## Observability Services

### Datadog Integration

```rust
use datadog_metrics::MetricClient;

pub struct DatadogSpecialist {
    client: Arc<MetricClient>,
}

impl DatadogSpecialist {
    pub fn new(api_key: &str) -> Self {
        let client = MetricClient::new(api_key.to_string());
        Self {
            client: Arc::new(client),
        }
    }

    pub async fn send_metric(&self, metric_name: &str, value: f64, tags: Vec<String>) -> Result<()> {
        self.client.gauge(metric_name, value, &tags).await?;
        Ok(())
    }

    pub async fn send_event(&self, title: &str, text: &str, tags: Vec<String>) -> Result<()> {
        self.client.event(title, text, &tags).await?;
        Ok(())
    }

    pub async fn send_trace(&self, trace: &str) -> Result<()> {
        self.client.trace(trace).await?;
        Ok(())
    }
}
```

### New Relic Integration

```rust
use newrelic::Client as NewRelicClient;

pub struct NewRelicSpecialist {
    client: Arc<NewRelicClient>,
}

impl NewRelicSpecialist {
    pub fn new(api_key: &str, account_id: &str) -> Self {
        let client = NewRelicClient::new(api_key.to_string(), account_id.to_string());
        Self {
            client: Arc::new(client),
        }
    }

    pub async fn record_metric(&self, name: &str, value: f64) -> Result<()> {
        self.client.record_metric(name, value).await?;
        Ok(())
    }

    pub async fn record_event(&self, event_type: &str, attributes: serde_json::Value) -> Result<()> {
        self.client.record_event(event_type, attributes).await?;
        Ok(())
    }
}
```

---

## Storage Services

### AWS S3 Integration

```rust
use aws_sdk_s3::{Client as S3Client, config::Region};

pub struct S3Specialist {
    client: Arc<S3Client>,
    bucket: String,
}

impl S3Specialist {
    pub async fn new(bucket: &str, region: &str) -> Result<Self> {
        let config = aws_config::from_env()
            .region(Region::new(region.to_string()))
            .load()
            .await;

        let client = S3Client::new(&config);

        Ok(Self {
            client: Arc::new(client),
            bucket: bucket.to_string(),
        })
    }

    pub async fn upload(&self, key: &str, body: Vec<u8>) -> Result<()> {
        self.client.put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(aws_sdk_s3::types::ByteStream::from(body))
            .send()
            .await?;

        Ok(())
    }

    pub async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let response = self.client.get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;

        let body = response.body.collect().await?;
        Ok(body.into_bytes().to_vec())
    }

    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<String>> {
        let response = self.client.list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await?;

        let keys = response.contents()
            .unwrap_or_default()
            .iter()
            .filter_map(|obj| obj.key().map(|k| k.to_string()))
            .collect();

        Ok(keys)
    }
}
```

### Google Cloud Storage Integration

```rust
use google_cloudstore::Client as GCSClient;

pub struct GCSSpecialist {
    client: Arc<GCSClient>,
    bucket: String,
}

impl GCSSpecialist {
    pub async fn new(bucket: &str, credentials_path: &str) -> Result<Self> {
        let client = GCSClient::from_file(credentials_path).await?;

        Ok(Self {
            client: Arc::new(client),
            bucket: bucket.to_string(),
        })
    }

    pub async fn upload(&self, object: &str, data: Vec<u8>) -> Result<()> {
        self.client.put_object(&self.bucket, object, data).await?;
        Ok(())
    }

    pub async fn download(&self, object: &str) -> Result<Vec<u8>> {
        self.client.get_object(&self.bucket, object).await
    }
}
```

---

## Databases

### MongoDB Integration

```rust
use mongodb::{Client, bson::doc};

pub struct MongoDBSpecialist {
    client: Arc<Client>,
    database: String,
}

impl MongoDBSpecialist {
    pub async fn new(connection_string: &str, database: &str) -> Result<Self> {
        let client = Client::with_uri_str(connection_string).await?;

        Ok(Self {
            client: Arc::new(client),
            database: database.to_string(),
        })
    }

    pub async fn insert(&self, collection: &str, document: serde_json::Value) -> Result<()> {
        let db = self.client.database(&self.database);
        let col = db.collection(collection);

        col.insert_one(document, None).await?;
        Ok(())
    }

    pub async fn query(&self, collection: &str, filter: serde_json::Value) -> Result<Vec<serde_json::Value>> {
        let db = self.client.database(&self.database);
        let col = db.collection(collection);

        let cursor = col.find(filter, None).await?;
        // Process cursor and collect results
        Ok(vec![])
    }
}
```

### Elasticsearch Integration

```rust
use elasticsearch::{Elasticsearch, http::transport::Transport};

pub struct ElasticsearchSpecialist {
    client: Arc<Elasticsearch>,
}

impl ElasticsearchSpecialist {
    pub async fn new(url: &str) -> Result<Self> {
        let transport = Transport::single_node(url)?;
        let client = Elasticsearch::new(transport);

        Ok(Self {
            client: Arc::new(client),
        })
    }

    pub async fn index_document(&self, index: &str, doc_type: &str, doc: serde_json::Value) -> Result<()> {
        self.client.index(elasticsearch::IndexParts::Index(index))
            .body(doc)
            .send()
            .await?;

        Ok(())
    }

    pub async fn search(&self, index: &str, query: serde_json::Value) -> Result<serde_json::Value> {
        let response = self.client.search(elasticsearch::SearchParts::Index(&[index]))
            .body(query)
            .send()
            .await?;

        Ok(response.json().await?)
    }
}
```

---

## Authentication Services

### Auth0 Integration

```rust
use auth0::Client as Auth0Client;

pub struct Auth0Specialist {
    client: Arc<Auth0Client>,
}

impl Auth0Specialist {
    pub fn new(domain: &str, client_id: &str, client_secret: &str) -> Self {
        let client = Auth0Client::new(
            domain.to_string(),
            client_id.to_string(),
            client_secret.to_string(),
        );

        Self {
            client: Arc::new(client),
        }
    }

    pub async fn create_user(&self, email: &str, password: &str) -> Result<String> {
        let user_id = self.client.create_user(email, password).await?;
        Ok(user_id)
    }

    pub async fn verify_token(&self, token: &str) -> Result<bool> {
        let valid = self.client.verify_token(token).await?;
        Ok(valid)
    }
}
```

### OAuth2 Integration

```rust
use oauth2::{Client, basic::BasicClient, AuthUrl, TokenUrl, RedirectUrl};

pub struct OAuth2Specialist {
    oauth_client: Arc<BasicClient>,
}

impl OAuth2Specialist {
    pub fn new(
        client_id: &str,
        client_secret: &str,
        auth_url: &str,
        token_url: &str,
        redirect_url: &str,
    ) -> Self {
        let oauth_client = BasicClient::new(
            oauth2::ClientId::new(client_id.to_string()),
            Some(oauth2::ClientSecret::new(client_secret.to_string())),
            AuthUrl::new(auth_url.to_string()).unwrap(),
            Some(TokenUrl::new(token_url.to_string()).unwrap()),
        ).set_redirect_uri(RedirectUrl::new(redirect_url.to_string()).unwrap());

        Self {
            oauth_client: Arc::new(oauth_client),
        }
    }

    pub fn get_auth_url(&self) -> String {
        let (auth_url, _) = self.oauth_client.authorize_url(|| {
            oauth2::CsrfToken::new_random()
        }).url();

        auth_url.to_string()
    }
}
```

---

## Webhooks & Event Systems

### Webhook Handler

```rust
use axum::{
    extract::{MatchedPath, Request},
    middleware::{self, Next},
    response::IntoResponse,
    routing::post,
    Router,
};

pub struct WebhookSpecialist {
    handlers: Arc<Mutex<HashMap<String, Box<dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<()>>>>>>,
}

impl WebhookSpecialist {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register_handler<F>(&self, event_type: &str, handler: F) -> Result<()>
    where
        F: Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<()>>>> + 'static,
    {
        let mut handlers = self.handlers.lock().unwrap();
        handlers.insert(event_type.to_string(), Box::new(handler));
        Ok(())
    }

    pub fn create_router(&self) -> Router {
        let handlers = Arc::clone(&self.handlers);

        Router::new()
            .route("/webhook", post(handle_webhook))
            .with_state(handlers)
    }
}

async fn handle_webhook(
    axum::extract::State(handlers): axum::extract::State<
        Arc<Mutex<HashMap<String, Box<dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = Result<()>>>>>
    >,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    let event_type = payload["type"].as_str().unwrap_or("unknown");
    
    if let Some(handler) = handlers.lock().unwrap().get(event_type) {
        match handler(payload).await {
            Ok(_) => "OK",
            Err(_) => "ERROR",
        }
    } else {
        "UNKNOWN"
    }
}
```

### Event Bus Integration

```rust
pub struct EventBusSpecialist {
    subscribers: Arc<Mutex<HashMap<String, Vec<Box<dyn Fn(serde_json::Value)>>>>>,
}

impl EventBusSpecialist {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn subscribe<F>(&self, event_type: &str, callback: F) -> Result<()>
    where
        F: Fn(serde_json::Value) + 'static,
    {
        let mut subs = self.subscribers.lock().unwrap();
        subs.entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(callback));
        Ok(())
    }

    pub fn emit(&self, event_type: &str, event: serde_json::Value) -> Result<()> {
        if let Some(callbacks) = self.subscribers.lock().unwrap().get(event_type) {
            for callback in callbacks {
                callback(event.clone());
            }
        }
        Ok(())
    }
}
```

---

## Integration Patterns

### Service Registry Pattern

```rust
pub struct ServiceRegistry {
    services: Arc<Mutex<HashMap<String, Box<dyn Any + Send + Sync>>>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register<T: 'static + Send + Sync>(&self, name: &str, service: T) -> Result<()> {
        self.services.lock().unwrap().insert(name.to_string(), Box::new(service));
        Ok(())
    }

    pub fn get<T: 'static + Send + Sync>(&self, name: &str) -> Result<Arc<T>> {
        let services = self.services.lock().unwrap();
        services.get(name)
            .and_then(|s| s.downcast_ref::<T>())
            .map(|t| Arc::new(t.clone()))
            .ok_or_else(|| SpecialistError::ServiceNotFound(name.to_string()).into())
    }
}
```

### Adapter Pattern for Multiple Services

```rust
pub trait ExternalServiceAdapter: Send + Sync {
    async fn connect(&self) -> Result<()>;
    async fn disconnect(&self) -> Result<()>;
    async fn health_check(&self) -> Result<bool>;
}

pub struct MultiServiceSpecialist {
    adapters: Arc<Mutex<HashMap<String, Box<dyn ExternalServiceAdapter>>>>,
}

impl MultiServiceSpecialist {
    pub fn new() -> Self {
        Self {
            adapters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_adapter(&self, name: &str, adapter: Box<dyn ExternalServiceAdapter>) -> Result<()> {
        adapter.connect().await?;
        self.adapters.lock().unwrap().insert(name.to_string(), adapter);
        Ok(())
    }

    pub async fn check_all_health(&self) -> Result<HashMap<String, bool>> {
        let adapters = self.adapters.lock().unwrap();
        let mut results = HashMap::new();

        for (name, adapter) in adapters.iter() {
            let healthy = adapter.health_check().await.unwrap_or(false);
            results.insert(name.clone(), healthy);
        }

        Ok(results)
    }
}
```

---

## Configuration Best Practices

### Environment-Based Configuration

```rust
pub struct IntegrationConfig {
    pub openai_api_key: String,
    pub pinecone_api_key: String,
    pub mongodb_url: String,
    pub kafka_brokers: String,
    pub s3_bucket: String,
    pub datadog_api_key: String,
}

impl IntegrationConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            openai_api_key: std::env::var("OPENAI_API_KEY")?,
            pinecone_api_key: std::env::var("PINECONE_API_KEY")?,
            mongodb_url: std::env::var("MONGODB_URL")?,
            kafka_brokers: std::env::var("KAFKA_BROKERS")?,
            s3_bucket: std::env::var("S3_BUCKET")?,
            datadog_api_key: std::env::var("DATADOG_API_KEY")?,
        })
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}
```

### Dependency Injection Pattern

```rust
pub struct ServiceContainer {
    config: Arc<IntegrationConfig>,
    llm_specialist: Arc<Mutex<Option<Box<dyn Specialist>>>>,
    vector_specialist: Arc<Mutex<Option<Box<dyn Specialist>>>>,
    storage_specialist: Arc<Mutex<Option<Box<dyn Specialist>>>>,
}

impl ServiceContainer {
    pub async fn initialize(&self) -> Result<()> {
        // Initialize LLM
        let llm = Box::new(LLMSpecialist::new(&self.config.openai_api_key, "gpt-4"));
        *self.llm_specialist.lock().unwrap() = Some(llm);

        // Initialize Vector Store
        let vector = Box::new(VectorStoreSpecialist::new(
            &self.config.pinecone_api_key,
            "default",
        ).await?);
        *self.vector_specialist.lock().unwrap() = Some(vector);

        // Initialize Storage
        let storage = Box::new(S3Specialist::new(&self.config.s3_bucket, "us-east-1").await?);
        *self.storage_specialist.lock().unwrap() = Some(storage);

        Ok(())
    }

    pub fn get_llm(&self) -> Result<Arc<Box<dyn Specialist>>> {
        self.llm_specialist.lock().unwrap()
            .as_ref()
            .cloned()
            .ok_or_else(|| "LLM not initialized".into())
    }
}
```

---

## Testing Integrations

### Mock Specialist for Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    pub struct MockExternalService;

    #[async_trait]
    impl Specialist for MockExternalService {
        fn id(&self) -> SpecialistId {
            SpecialistId::from("mock-external")
        }

        fn name(&self) -> &str {
            "Mock External Service"
        }

        fn capabilities(&self) -> Vec<String> {
            vec!["test".to_string()]
        }

        async fn propose(&self, _context: &Context) -> Result<Proposal> {
            Ok(Proposal {
                proposal_id: "test-proposal".to_string(),
                specialist_id: self.id(),
                solution: Default::default(),
                confidence: 1.0,
                estimated_cost: Default::default(),
                dependencies: vec![],
                alternatives: vec![],
                metadata: Default::default(),
            })
        }

        // ... implement other methods ...
    }

    #[tokio::test]
    async fn test_external_integration() {
        let mock = MockExternalService;
        let context = create_test_context();

        let proposal = mock.propose(&context).await.unwrap();
        assert_eq!(proposal.confidence, 1.0);
    }
}
```

---

## Summary

This guide provides:

✅ **Vector Database** integration (Pinecone, Weaviate)
✅ **LLM** integration (OpenAI, Anthropic, Ollama)
✅ **Message Queues** (RabbitMQ, Kafka)
✅ **Observability** (Datadog, New Relic)
✅ **Cloud Storage** (AWS S3, GCS)
✅ **Databases** (MongoDB, Elasticsearch)
✅ **Authentication** (Auth0, OAuth2)
✅ **Webhooks** and event systems
✅ **Design Patterns** (service registry, adapters)
✅ **Testing** and mocking

---

**Ready to integrate with external services! 🔗**
