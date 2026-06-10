//! Throwaway output sink for the CDK / S3 experiment.
//!
//! When `SCHISM_S3_BUCKET` is set, the final readout JSON is uploaded to that
//! bucket at the end of a run. Region and credentials come from the standard AWS
//! provider chain (env vars / shared config / ECS task role) — same as boto3, so
//! nothing AWS-specific has to be passed in.
//!
//! This whole module is meant to be deleted once the experiment is over. To rip
//! it out: remove `mod output;` from `main.rs`, the single
//! `output::upload_if_configured(...)` call in `simulation::run`, and the
//! `aws-*` / `tokio` deps from `Cargo.toml`.

use std::env;

use anyhow::{Context, Result};

/// Env var naming the destination bucket. Unset (or empty) => upload is skipped.
const BUCKET_ENV: &str = "SCHISM_S3_BUCKET";
/// Env var overriding the object key. Optional; see [`default_key`].
const KEY_ENV: &str = "SCHISM_S3_KEY";

/// Upload `readout_json` to S3 when `SCHISM_S3_BUCKET` is set; otherwise a no-op
/// so local runs are unaffected. Errors are surfaced to the caller. Touches no
/// simulation state.
pub fn upload_if_configured(readout_json: &str) -> Result<()> {
    let destination_bucket = match env::var(BUCKET_ENV) {
        Ok(bucket) if !bucket.is_empty() => bucket,
        _ => return Ok(()), // not configured: nothing to do
    };

    let object_key = env::var(KEY_ENV).unwrap_or_else(|_| default_key());

    // The AWS SDK is async; spin up a small current-thread runtime just for this
    // one upload so the rest of the engine can stay synchronous.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building tokio runtime for s3 upload")?;

    runtime.block_on(upload(&destination_bucket, &object_key, readout_json))?;

    eprintln!("uploaded readout to s3://{destination_bucket}/{object_key}");
    Ok(())
}

/// Default object key when `SCHISM_S3_KEY` is unset: namespaced and stamped with
/// the unix time so repeated runs don't clobber each other.
fn default_key() -> String {
    let seconds_since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    format!("schism/readout-{seconds_since_epoch}.json")
}

async fn upload(bucket: &str, key: &str, readout_json: &str) -> Result<()> {
    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = aws_sdk_s3::Client::new(&aws_config);

    let body = aws_sdk_s3::primitives::ByteStream::from(readout_json.as_bytes().to_vec());

    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .content_type("application/json")
        .send()
        .await
        .with_context(|| format!("uploading readout to s3://{bucket}/{key}"))?;

    Ok(())
}
