use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response, StatusCode},
};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tower::{Layer, Service};

/// Rate limit store using a simple [HashMap] for storing, mainly intended for low traffic routes.
#[derive(Clone, Debug, Default)]
pub struct RateLimitStore {
    inner: Arc<Mutex<HashMap<IpAddr, (u32, Instant)>>>,
}

impl RateLimitStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn is_allowed(&self, ip: IpAddr, limit: u32, window: Duration) -> bool {
        let mut map = self.inner.lock().await;
        let now = Instant::now();

        let entry = map.entry(ip).or_insert((0, now));

        if now.duration_since(entry.1) >= window {
            *entry = (1, now);
            return true;
        }

        if entry.0 < limit {
            entry.0 += 1;
            true
        } else {
            false
        }
    }
}

#[derive(Clone)]
pub struct IpRateLimitLayer {
    store: RateLimitStore,
    limit: u32,
    window: Duration,
}

impl IpRateLimitLayer {
    pub fn new(store: RateLimitStore, limit: u32, window: Duration) -> Self {
        Self {
            store,
            limit,
            window,
        }
    }
}

impl<S> Layer<S> for IpRateLimitLayer {
    type Service = IpRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IpRateLimitService {
            inner,
            store: self.store.clone(),
            limit: self.limit,
            window: self.window,
        }
    }
}

#[derive(Clone)]
pub struct IpRateLimitService<S> {
    inner: S,
    store: RateLimitStore,
    limit: u32,
    window: Duration,
}

impl<S> Service<Request<Body>> for IpRateLimitService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let store = self.store.clone();
        let limit = self.limit;
        let window = self.window;

        let ip = req
            .extensions()
            .get::<ConnectInfo<std::net::SocketAddr>>()
            .map(|ci| ci.0.ip());

        let clone = self.inner.clone();
        let mut ready_inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let allowed = match ip {
                Some(ip) => store.is_allowed(ip, limit, window).await,
                None => false, // deny if IP unresolvable
            };

            if !allowed {
                return Ok(Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(Body::empty())
                    .unwrap());
            }

            ready_inner.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::Duration;

    #[tokio::test]
    async fn test_rate_limit_store_creation() {
        let store = RateLimitStore::new();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First request should always be allowed
        assert!(store.is_allowed(ip, 5, Duration::from_secs(60)).await);
    }

    #[tokio::test]
    async fn test_allow_requests_up_to_limit() {
        let store = RateLimitStore::new();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let limit = 5;
        let window = Duration::from_secs(60);

        // Should allow exactly 'limit' requests
        for i in 0..limit {
            assert!(
                store.is_allowed(ip, limit, window).await,
                "Request {} should be allowed",
                i + 1
            );
        }
    }

    #[tokio::test]
    async fn test_deny_requests_exceeding_limit() {
        let store = RateLimitStore::new();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let limit = 3;
        let window = Duration::from_secs(60);

        // Allow up to limit
        for _ in 0..limit {
            store.is_allowed(ip, limit, window).await;
        }

        // Next request should be denied
        assert!(!store.is_allowed(ip, limit, window).await);
        assert!(!store.is_allowed(ip, limit, window).await);
    }

    #[tokio::test]
    async fn test_multiple_ips_tracked_independently() {
        let store = RateLimitStore::new();
        let ip1 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let limit = 2;
        let window = Duration::from_secs(60);

        // Use up limit for ip1
        store.is_allowed(ip1, limit, window).await;
        store.is_allowed(ip1, limit, window).await;

        // ip2 should still have requests available
        assert!(store.is_allowed(ip2, limit, window).await);
        assert!(store.is_allowed(ip2, limit, window).await);
        assert!(!store.is_allowed(ip2, limit, window).await);

        // ip1 should still be denied
        assert!(!store.is_allowed(ip1, limit, window).await);
    }

    #[tokio::test]
    async fn test_window_expiration_resets_counter() {
        let store = RateLimitStore::new();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let limit = 2;
        let window = Duration::from_millis(100);

        // Use up the limit
        store.is_allowed(ip, limit, window).await;
        store.is_allowed(ip, limit, window).await;

        // Next request should be denied
        assert!(!store.is_allowed(ip, limit, window).await);

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // After window expires, counter should reset
        assert!(store.is_allowed(ip, limit, window).await);
        assert!(store.is_allowed(ip, limit, window).await);
        assert!(!store.is_allowed(ip, limit, window).await);
    }

    #[tokio::test]
    async fn test_zero_limit() {
        let store = RateLimitStore::new();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let limit = 0;
        let window = Duration::from_secs(60);

        // With zero limit, even first request should be denied
        assert!(!store.is_allowed(ip, limit, window).await);
    }

    #[tokio::test]
    async fn test_concurrent_requests_same_ip() {
        let store = RateLimitStore::new();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let limit = 10;
        let window = Duration::from_secs(60);

        // Spawn multiple concurrent tasks making requests
        let mut handles = vec![];
        for _ in 0..15 {
            let store_clone = store.clone();
            let handle =
                tokio::spawn(async move { store_clone.is_allowed(ip, limit, window).await });
            handles.push(handle);
        }

        // Collect results
        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        // Count allowed (should be exactly 'limit')
        let allowed_count = results.iter().filter(|&&r| r).count();
        assert_eq!(
            allowed_count, limit as usize,
            "Expected exactly {} allowed requests, got {}",
            limit, allowed_count
        );

        // Count denied (should be 15 - limit)
        let denied_count = results.iter().filter(|&&r| !r).count();
        assert_eq!(denied_count, 15 - limit as usize);
    }

    #[tokio::test]
    async fn test_store_clone_shares_state() {
        let store1 = RateLimitStore::new();
        let store2 = store1.clone();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let limit = 2;
        let window = Duration::from_secs(60);

        // Make requests through first store
        store1.is_allowed(ip, limit, window).await;
        store1.is_allowed(ip, limit, window).await;

        // Cloned store should see the same state
        assert!(!store2.is_allowed(ip, limit, window).await);
    }
}
