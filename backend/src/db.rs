use mongodb::{bson::doc, options::{ClientOptions, ServerApi, ServerApiVersion}, Client, Database};
use std::env;

pub async fn connect() -> Database {
    let uri = env::var("MONGODB_URI").expect("MONGODB_URI must be set in .env");

    let mut client_options = ClientOptions::parse(&uri)
        .await
        .expect("Failed to parse MongoDB URI");

    // Set the server_api field of the client_options object to set the version of the Stable API on the client
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);

    // Equivalent to bufferCommands: false — do not queue commands if not connected
    client_options.connect_timeout = Some(std::time::Duration::from_secs(5));

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
