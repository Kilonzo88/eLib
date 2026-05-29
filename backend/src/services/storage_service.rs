use anyhow::Result;
use aws_sdk_s3::Client;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::config::Region;

/// Ensures the R2 bucket has CORS configured to allow browser-side fetching.
/// Called once during backend startup.
pub async fn ensure_cors_config() -> Result<()> {
    let account_id = std::env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set");
    let access_key = std::env::var("R2_ACCESS_KEY").expect("R2_ACCESS_KEY must be set");
    let secret_key = std::env::var("R2_SECRET_KEY").expect("R2_SECRET_KEY must be set");
    let bucket_name = std::env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set");
    
    let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);

    let region_provider = RegionProviderChain::default_provider().or_else(Region::new("auto"));
    let credentials = aws_sdk_s3::config::Credentials::new(
        &access_key,
        &secret_key,
        None,
        None,
        "r2",
    );

    let config = aws_sdk_s3::config::Builder::new()
        .region(region_provider.region().await)
        .endpoint_url(&endpoint_url)
        .credentials_provider(credentials)
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::v2026_01_12())
        .force_path_style(true)
        .build();

    let client = Client::from_conf(config);

    use aws_sdk_s3::types::{CorsConfiguration, CorsRule};

    let cors_rule = CorsRule::builder()
        .set_allowed_origins(Some(vec!["*".to_string()]))
        .set_allowed_methods(Some(vec!["GET".to_string(), "HEAD".to_string()]))
        .set_allowed_headers(Some(vec!["*".to_string()]))
        .set_expose_headers(Some(vec!["Content-Length".to_string(), "Content-Type".to_string()]))
        .max_age_seconds(3600)
        .build()?;

    let cors_config = CorsConfiguration::builder()
        .cors_rules(cors_rule)
        .build()?;

    client
        .put_bucket_cors()
        .bucket(&bucket_name)
        .cors_configuration(cors_config)
        .send()
        .await?;

    println!("R2 CORS configuration applied successfully.");
    Ok(())
}

pub async fn upload_to_r2(
    file_path: &std::path::Path, 
    clerk_id: &str, 
    slug: &str, 
    ext: &str
) -> Result<(String, String)> {
    let account_id = std::env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set");
    let access_key = std::env::var("R2_ACCESS_KEY").expect("R2_ACCESS_KEY must be set");
    let secret_key = std::env::var("R2_SECRET_KEY").expect("R2_SECRET_KEY must be set");
    let bucket_name = std::env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set");
    let public_url_domain = std::env::var("R2_PUBLIC_URL_DOMAIN").expect("R2_PUBLIC_URL_DOMAIN must be set");
    
    let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);

    let region_provider = RegionProviderChain::default_provider().or_else(Region::new("auto"));
    let credentials = aws_sdk_s3::config::Credentials::new(
        &access_key,
        &secret_key,
        None,
        None,
        "r2",
    );

    let config = aws_sdk_s3::config::Builder::new()
        .region(region_provider.region().await)
        .endpoint_url(&endpoint_url)
        .credentials_provider(credentials)
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::v2026_01_12())
        .force_path_style(true)
        .build();

    let client = Client::from_conf(config);

    // Deterministic path: users/{clerk_id}/{slug}/book.{ext}
    // Gutenberg falls back to gutenberg/{id}/book.epub
    let r2_key = if clerk_id == "public" {
        let id = slug.replace("gutenberg-", "");
        format!("gutenberg/{}/book.{}", id, ext)
    } else {
        format!("users/{}/{}/book.{}", clerk_id, slug, ext)
    };

    let byte_stream = aws_sdk_s3::primitives::ByteStream::from_path(file_path).await?;
    let content_type = if ext == "epub" { "application/epub+zip" } else { "application/pdf" };
    
    client
        .put_object()
        .bucket(&bucket_name)
        .key(&r2_key)
        .body(byte_stream)
        .content_type(content_type)
        .send()
        .await?;

    // Return the public URL and the unique key used in R2
    let file_url = format!("{}/{}", public_url_domain, r2_key);
    Ok((file_url, r2_key))
}

pub async fn upload_image_to_r2(
    image_data: &[u8], 
    clerk_id: &str, 
    slug: &str
) -> Result<(String, String)> {
    let account_id = std::env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set");
    let access_key = std::env::var("R2_ACCESS_KEY").expect("R2_ACCESS_KEY must be set");
    let secret_key = std::env::var("R2_SECRET_KEY").expect("R2_SECRET_KEY must be set");
    let bucket_name = std::env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set");
    let public_url_domain = std::env::var("R2_PUBLIC_URL_DOMAIN").expect("R2_PUBLIC_URL_DOMAIN must be set");
    
    let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);

    let region_provider = RegionProviderChain::default_provider().or_else(Region::new("auto"));
    let credentials = aws_sdk_s3::config::Credentials::new(
        &access_key,
        &secret_key,
        None,
        None,
        "r2",
    );

    let config = aws_sdk_s3::config::Builder::new()
        .region(region_provider.region().await)
        .endpoint_url(&endpoint_url)
        .credentials_provider(credentials)
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::v2026_01_12())
        .force_path_style(true)
        .build();

    let client = Client::from_conf(config);

    // Deterministic cover path
    let r2_key = if clerk_id == "public" {
        let id = slug.replace("gutenberg-", "");
        format!("gutenberg/{}/cover.png", id)
    } else {
        format!("users/{}/{}/cover.png", clerk_id, slug)
    };

    client
        .put_object()
        .bucket(&bucket_name)
        .key(&r2_key)
        .body(image_data.to_vec().into())
        .content_type("image/png")
        .send()
        .await?;

    // Return the public URL and the unique key used in R2
    let file_url = format!("{}/{}", public_url_domain, r2_key);
    Ok((file_url, r2_key))
}

pub async fn get_file_from_r2(key: &str) -> Result<Vec<u8>> {
    let account_id = std::env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set");
    let access_key = std::env::var("R2_ACCESS_KEY").expect("R2_ACCESS_KEY must be set");
    let secret_key = std::env::var("R2_SECRET_KEY").expect("R2_SECRET_KEY must be set");
    let bucket_name = std::env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set");
    
    let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);

    let region_provider = RegionProviderChain::default_provider().or_else(Region::new("auto"));
    let credentials = aws_sdk_s3::config::Credentials::new(
        &access_key,
        &secret_key,
        None,
        None,
        "r2",
    );

    let config = aws_sdk_s3::config::Builder::new()
        .region(region_provider.region().await)
        .endpoint_url(&endpoint_url)
        .credentials_provider(credentials)
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::v2026_01_12())
        .force_path_style(true)
        .build();

    let client = Client::from_conf(config);

    let res = client
        .get_object()
        .bucket(&bucket_name)
        .key(key)
        .send()
        .await?;

    let data = res.body.collect().await?.to_vec();
    Ok(data)
}

pub async fn generate_presigned_url(key: &str) -> Result<String> {
    let account_id = std::env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set");
    let access_key = std::env::var("R2_ACCESS_KEY").expect("R2_ACCESS_KEY must be set");
    let secret_key = std::env::var("R2_SECRET_KEY").expect("R2_SECRET_KEY must be set");
    let bucket_name = std::env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set");
    
    let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);

    let region_provider = RegionProviderChain::default_provider().or_else(Region::new("auto"));
    let credentials = aws_sdk_s3::config::Credentials::new(
        &access_key,
        &secret_key,
        None,
        None,
        "r2",
    );

    let config = aws_sdk_s3::config::Builder::new()
        .region(region_provider.region().await)
        .endpoint_url(&endpoint_url)
        .credentials_provider(credentials)
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::v2026_01_12())
        .force_path_style(true)
        .build();

    let client = Client::from_conf(config);

    // Use the presigning config to create a temporary URL (default 1 hour)
    let expires_in = std::time::Duration::from_secs(3600);
    let presigning_config = aws_sdk_s3::presigning::PresigningConfig::builder()
        .expires_in(expires_in)
        .build()?;

    let req = client
        .get_object()
        .bucket(&bucket_name)
        .key(key)
        .presigned(presigning_config)
        .await?;

    Ok(req.uri().to_string())
}

pub async fn validate_config() -> Result<()> {
    let account_id = std::env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set");
    let access_key = std::env::var("R2_ACCESS_KEY").expect("R2_ACCESS_KEY must be set");
    let secret_key = std::env::var("R2_SECRET_KEY").expect("R2_SECRET_KEY must be set");
    let bucket_name = std::env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set");
    
    let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);

    let region_provider = RegionProviderChain::default_provider().or_else(Region::new("auto"));
    let credentials = aws_sdk_s3::config::Credentials::new(
        &access_key,
        &secret_key,
        None,
        None,
        "r2",
    );

    let config = aws_sdk_s3::config::Builder::new()
        .region(region_provider.region().await)
        .endpoint_url(&endpoint_url)
        .credentials_provider(credentials)
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::v2026_01_12())
        .force_path_style(true)
        .build();

    let client = Client::from_conf(config);

    // Try to list objects (limit 1) to verify connectivity and permissions
    client
        .list_objects_v2()
        .bucket(&bucket_name)
        .max_keys(1)
        .send()
        .await?;

    Ok(())
}

pub async fn upload_bytes(key: &str, data: &[u8], content_type: &str) -> Result<()> {
    let account_id = std::env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set");
    let access_key = std::env::var("R2_ACCESS_KEY").expect("R2_ACCESS_KEY must be set");
    let secret_key = std::env::var("R2_SECRET_KEY").expect("R2_SECRET_KEY must be set");
    let bucket_name = std::env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set");
    
    let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", account_id);

    let region_provider = RegionProviderChain::default_provider().or_else(Region::new("auto"));
    let credentials = aws_sdk_s3::config::Credentials::new(
        &access_key,
        &secret_key,
        None,
        None,
        "r2",
    );

    let config = aws_sdk_s3::config::Builder::new()
        .region(region_provider.region().await)
        .endpoint_url(&endpoint_url)
        .credentials_provider(credentials)
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::v2026_01_12())
        .force_path_style(true)
        .build();

    let client = Client::from_conf(config);

    client
        .put_object()
        .bucket(&bucket_name)
        .key(key)
        .body(data.to_vec().into())
        .content_type(content_type)
        .send()
        .await?;

    Ok(())
}
