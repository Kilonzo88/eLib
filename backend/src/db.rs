use mongodb::{bson::{doc, Document}, options::{ClientOptions, ServerApi, ServerApiVersion}, Client, Database};
use std::env;

fn is_ready_vector_search_index(index: &Document) -> bool {
    index.get_str("name").ok() == Some("vector_index")
        && index.get_str("type").ok() == Some("vectorSearch")
        && index.get_str("status").ok() == Some("READY")
        && index.get_bool("queryable").unwrap_or(false)
}

pub async fn vector_search_ready(db: &Database) -> bool {
    let segments = db.collection::<mongodb::bson::Document>("book_segments");

    let mut indexes = match segments.list_search_indexes().await {
        Ok(indexes) => indexes,
        Err(e) => {
            eprintln!("[db] Unable to list Atlas Search indexes: {:?}", e);
            return false;
        }
    };

    loop {
        match indexes.advance().await {
            Ok(true) => {
                let index: Document = match indexes.deserialize_current() {
                    Ok(index) => index,
                    Err(e) => {
                        eprintln!("[db] Unable to read Atlas Search index: {:?}", e);
                        return false;
                    }
                };

                if is_ready_vector_search_index(&index) {
                    return true;
                }
            }
            Ok(false) => break,
            Err(e) => {
                eprintln!("[db] Unable to read Atlas Search indexes: {:?}", e);
                return false;
            }
        }
    }

    eprintln!("[db] Atlas Vector Search index 'vector_index' is missing or not queryable on book_segments.");
    false
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
    let client = Client::with_options(client_options)
        .expect("Failed to create MongoDB client");

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
    use super::is_ready_vector_search_index;
    use mongodb::bson::doc;

    #[test]
    fn detects_ready_vector_search_index() {
        let index = doc! {
            "name": "vector_index",
            "type": "vectorSearch",
            "status": "READY",
            "queryable": true,
        };
        assert!(is_ready_vector_search_index(&index));
    }

    #[test]
    fn rejects_wrong_type_and_not_ready_vector_indexes() {
        let wrong_type = doc! {
            "name": "vector_index",
            "type": "search",
            "status": "READY",
            "queryable": true,
        };
        let building = doc! {
            "name": "vector_index",
            "type": "vectorSearch",
            "status": "BUILDING",
            "queryable": false,
        };
        let other_name = doc! {
            "name": "other_index",
            "type": "vectorSearch",
            "status": "READY",
            "queryable": true,
        };

        assert!(!is_ready_vector_search_index(&wrong_type));
        assert!(!is_ready_vector_search_index(&building));
        assert!(!is_ready_vector_search_index(&other_name));
    }
}
