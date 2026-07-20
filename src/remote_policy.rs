use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};
use crate::policy_config::PolicyConfig;

/// Creates a new policy update endpoint.
pub fn policy_update_route(
    policy: Arc<RwLock<PolicyConfig>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("policy"))
        .and(warp::body::json())
        .and(with_policy(policy))
        .and_then(update_policy_handler)
}

fn with_policy(
    policy: Arc<RwLock<PolicyConfig>>,
) -> impl Filter<Extract = (Arc<RwLock<PolicyConfig>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || policy.clone())
}

async fn update_policy_handler(
    new_policy: PolicyConfig,
    policy: Arc<RwLock<PolicyConfig>>,
) -> Result<impl Reply, Rejection> {
    let mut policy = policy.write().await;
    *policy = new_policy;
    Ok(warp::reply::json(&*policy))
}
