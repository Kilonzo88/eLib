use anyhow::Result;
use aws_sdk_s3::Client;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::config::Region;

pub async fn upload_pdf_to_r2(pdf_data: &[u8], filename: &str) -> Result<String> {
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
        .build();

    let client = Client::from_conf(config);

    // Add unique identifier to the filename to prevent collisions and replaced spaces with underscores
    let unique_filename = format!("{}-{}", uuid::Uuid::new_v4(), filename.replace(" ", "_"));

    client
        .put_object()
        .bucket(&bucket_name)
        .key(&unique_filename)
        .body(pdf_data.to_vec().into())
        .content_type("application/pdf")
        .send()
        .await?;

    // Return the public URL
    let file_url = format!("{}/{}", public_url_domain, unique_filename);
    Ok(file_url)
}
