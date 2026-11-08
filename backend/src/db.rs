use mongodb::{
    Client, Database,
    bson::{Document, doc, oid::ObjectId},
    options::{ClientOptions, ServerApi, ServerApiVersion},
};
use std::env;

/// Name of the Atlas Vector Search index on `book_segments`.
pub const VECTOR_INDEX_NAME: &str = "vector_index";

/// Text field that Atlas Automated Embeddings indexes.
///
/// With Automated Embeddings, MongoDB (not this backend) calls Voyage AI to embed
/// this field at index time, on document updates, and for the text supplied to
/// `$vectorSearch.query`. The application never stores a vector itself.
pub const VECTOR_TEXT_FIELD: &str = "content";

/// Voyage AI embedding model configured on the index.
///
/// `voyage-4` is the general-purpose model in MongoDB's managed Voyage lineup.
pub const EMBEDDING_MODEL: &str = "voyage-4";

/// `voyage-4` default output dimension. It must match the index definition.
pub const EMBEDDING_DIMENSIONS: i32 = 1024;

/// Definition for the `autoEmbed` Atlas Vector Search index.
///
/// `book_id` and `clerk_id` are declared as `filter` fields because chat scopes every
/// query to one book and one owner. Atlas rejects (or silently returns nothing for)
/// filters on fields that are not indexed for pre-filtering.
pub fn vector_index_definition() -> Document {
    doc! {
        "fields": [
            {
                "type": "autoEmbed",
                "modality": "text",
                "path": VECTOR_TEXT_FIELD,
                "model": EMBEDDING_MODEL,
                "numDimensions": EMBEDDING_DIMENSIONS,
                "similarity": "cosine",
            },
            { "type": "filter", "path": "book_id" },
            { "type": "filter", "path": "clerk_id" },
        ]
    }
}

/// Builds the `$vectorSearch` stage for a natural-language query.
///
/// Automated Embeddings accepts `query: { text: ... }` instead of `queryVector`.
/// `path` must be the field declared as `autoEmbed` (not a stored embedding field).
pub fn vector_search_stage(query: &str, book_id: ObjectId, clerk_id: &str) -> Document {
    doc! {
        "$vectorSearch": {
            "index": VECTOR_INDEX_NAME,
            "path": VECTOR_TEXT_FIELD,
            "query": { "text": query },
            "numCandidates": 100,
            "limit": 4, // retrieve top 4 segments
            "filter": {
                "book_id": book_id,
                "clerk_id": clerk_id,
            }
        }
    }
}

fn index_definition(index: &Document) -> Option<&Document> {
    index
        .get_document("latestDefinition")
        .or_else(|_| index.get_document("definition"))
        .ok()
}

fn has_auto_embed_field(definition: &Document) -> bool {
    definition
        .get_array("fields")
        .ok()
        .map(|fields| {
            fields.iter().any(|field| {
                field.as_document().is_some_and(|field| {
                    field.get_str("type").ok() == Some("autoEmbed")
                        && field.get_str("path").ok() == Some(VECTOR_TEXT_FIELD)
                        && field
                            .get_str("model")
                            .ok()
                            .is_some_and(|model| model.starts_with("voyage-4"))
                })
            })
        })
        .unwrap_or(false)
}

/// True when the index has the shape chat requires: a `vectorSearch` index named
/// `vector_index` with an `autoEmbed` field on `content`.
///
/// This deliberately checks the definition, not just the name. An older `vector`
/// index on `embedding` would otherwise satisfy a name-only check and make every
/// query return zero results without an error.
pub fn is_compatible_vector_index(index: &Document) -> bool {
    index.get_str("name").ok() == Some(VECTOR_INDEX_NAME)
        && index.get_str("type").ok() == Some("vectorSearch")
        && index_definition(index).is_some_and(has_auto_embed_field)
}

fn is_ready_vector_search_index(index: &Document) -> bool {
    is_compatible_vector_index(index)
        && index.get_str("status").ok() == Some("READY")
        && index.get_bool("queryable").unwrap_or(false)
}

/// Lists Atlas Search / Vector Search indexes for `book_segments`.
pub async fn list_search_indexes(db: &Database) -> mongodb::error::Result<Vec<Document>> {
    let segments = db.collection::<Document>("book_segments");
    let mut cursor = segments.list_search_indexes().await?;
    let mut indexes = Vec::new();

    while cursor.advance().await? {
        indexes.push(cursor.deserialize_current()?);
    }

    Ok(indexes)
}

/// Checks that a queryable `autoEmbed` index exists for chat retrieval.
pub async fn vector_search_ready(db: &Database) -> bool {
    match list_search_indexes(db).await {
        Ok(indexes) => {
            if indexes.iter().any(is_ready_vector_search_index) {
                return true;
            }

            for index in &indexes {
                eprintln!("[db] Found search index: {:?}", index);
            }
            eprintln!(
                "[db] Atlas Vector Search index '{}' is missing, not queryable, or is not an \
                 autoEmbed index on '{}'. Run `cargo run --bin setup_vector_index` and wait for \
                 the index to become READY.",
                VECTOR_INDEX_NAME, VECTOR_TEXT_FIELD
            );
            false
        }
        Err(e) => {
            eprintln!("[db] Unable to list Atlas Search indexes: {:?}", e);
            false
        }
    }
}

pub async fn connect() -> Database {
    let uri = env::var("MONGODB_URI").expect("MONGODB_URI must be set in .env");

    let mut client_options = ClientOptions::parse(&uri)
        .await
        .expect("Failed to parse MongoDB URI");

    // Set the server_api field of the client_options object to set the version of the Stable API on the client
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);

    // Equivalent to bufferCommands: false — do not queue commands if not connected
    // Increasing from 5s to 30s to accommodate MongoDB Atlas free tier intermittent connection delays
    client_options.connect_timeout = Some(std::time::Duration::from_secs(30));

    // Get a handle to the cluster
    let client = Client::with_options(client_options).expect("Failed to create MongoDB client");

    // Ping the server to see if you can connect to the cluster
    client
        .database("admin")
        .run_command(doc! {"ping": 1})
        .await
        .expect("Failed to ping MongoDB server");

    println!("Pinged your deployment. You successfully connected to MongoDB!");

    let db_name = env::var("MONGODB_DB").unwrap_or_else(|_| "elib".to_string());
    client.database(&db_name)
}

#[cfg(test)]
mod tests {
    use super::{
        EMBEDDING_DIMENSIONS, EMBEDDING_MODEL, VECTOR_INDEX_NAME, VECTOR_TEXT_FIELD,
        is_compatible_vector_index, is_ready_vector_search_index, vector_index_definition,
        vector_search_stage,
    };
    use mongodb::bson::{doc, oid::ObjectId};

    fn ready_index(definition: mongodb::bson::Document) -> mongodb::bson::Document {
        doc! {
            "name": VECTOR_INDEX_NAME,
            "type": "vectorSearch",
            "status": "READY",
            "queryable": true,
            "latestDefinition": definition,
        }
    }

    #[test]
    fn index_definition_uses_voyage_auto_embed_on_content() {
        let definition = vector_index_definition();
        let fields = definition.get_array("fields").expect("fields array");

        let auto_embed = fields[0].as_document().expect("autoEmbed document");
        assert_eq!(auto_embed.get_str("type").unwrap(), "autoEmbed");
        assert_eq!(auto_embed.get_str("modality").unwrap(), "text");
        assert_eq!(auto_embed.get_str("path").unwrap(), VECTOR_TEXT_FIELD);
        assert_eq!(auto_embed.get_str("model").unwrap(), EMBEDDING_MODEL);
        assert_eq!(
            auto_embed.get_i32("numDimensions").unwrap(),
            EMBEDDING_DIMENSIONS
        );

        assert!(fields.iter().skip(1).all(|field| {
            let field = field.as_document().expect("filter document");
            field.get_str("type").unwrap() == "filter"
        }));
    }

    #[test]
    fn detects_ready_auto_embed_index() {
        let index = ready_index(vector_index_definition());
        assert!(is_compatible_vector_index(&index));
        assert!(is_ready_vector_search_index(&index));
    }

    #[test]
    fn rejects_legacy_vector_index_on_embedding() {
        let index = ready_index(doc! {
            "fields": [
                {
                    "type": "vector",
                    "path": "embedding",
                    "numDimensions": 768,
                    "similarity": "cosine",
                }
            ]
        });

        assert!(!is_compatible_vector_index(&index));
        assert!(!is_ready_vector_search_index(&index));
    }

    #[test]
    fn rejects_index_that_is_building_or_not_queryable() {
        let building = doc! {
            "name": VECTOR_INDEX_NAME,
            "type": "vectorSearch",
            "status": "BUILDING",
            "queryable": false,
            "latestDefinition": vector_index_definition(),
        };
        let wrong_name = doc! {
            "name": "other_index",
            "type": "vectorSearch",
            "status": "READY",
            "queryable": true,
            "latestDefinition": vector_index_definition(),
        };
        let wrong_type = doc! {
            "name": VECTOR_INDEX_NAME,
            "type": "search",
            "status": "READY",
            "queryable": true,
            "latestDefinition": vector_index_definition(),
        };

        assert!(!is_ready_vector_search_index(&building));
        assert!(!is_ready_vector_search_index(&wrong_name));
        assert!(!is_ready_vector_search_index(&wrong_type));
    }

    #[test]
    fn accepts_definition_key_as_well_as_latest_definition() {
        let index = doc! {
            "name": VECTOR_INDEX_NAME,
            "type": "vectorSearch",
            "status": "READY",
            "queryable": true,
            "definition": vector_index_definition(),
        };

        assert!(is_ready_vector_search_index(&index));
    }

    #[test]
    fn query_stage_uses_text_query_not_query_vector() {
        let book_id = ObjectId::new();
        let stage = vector_search_stage("who is the narrator?", book_id, "user_123");
        let search = stage.get_document("$vectorSearch").expect("stage");

        assert_eq!(search.get_str("index").unwrap(), VECTOR_INDEX_NAME);
        assert_eq!(search.get_str("path").unwrap(), VECTOR_TEXT_FIELD);
        assert_eq!(
            search
                .get_document("query")
                .unwrap()
                .get_str("text")
                .unwrap(),
            "who is the narrator?"
        );
        assert!(!search.contains_key("queryVector"));

        let filter = search.get_document("filter").expect("filter");
        assert_eq!(filter.get_object_id("book_id").unwrap(), book_id);
        assert_eq!(filter.get_str("clerk_id").unwrap(), "user_123");
    }
}
