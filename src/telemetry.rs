use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use warp::{Filter, Rejection, Reply};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Initializes the telemetry system.
/// This sets up a Prometheus exporter.
pub fn init_telemetry() -> Result<Arc<Mutex<PrometheusHandle>>, Box<dyn std::error::Error>> {
    let builder = PrometheusBuilder::new();
    let handle = builder.install_recorder()?;
    Ok(Arc::new(Mutex::new(handle)))
}

/// Returns a Warp filter for the /metrics endpoint.
pub fn metrics_route(handle: Arc<Mutex<PrometheusHandle>>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("metrics").and(warp::any().map(move || handle.clone())).and_then(metrics_handler)
}

async fn metrics_handler(handle: Arc<Mutex<PrometheusHandle>>) -> Result<impl Reply, Rejection> {
    let handle = handle.lock().await;
    let metrics = handle.render();
    Ok(metrics)
}
