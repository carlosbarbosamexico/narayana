use metrics::{counter, histogram, gauge};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Metrics {
    pub queries_total: Arc<RwLock<u64>>,
    pub queries_duration: Arc<RwLock<Vec<f64>>>,
    pub rows_inserted_total: Arc<RwLock<u64>>,
    pub rows_read_total: Arc<RwLock<u64>>,
    pub active_connections: Arc<RwLock<u64>>,
}

impl Metrics {
    pub fn new() -> Self {
        // In production, would install Prometheus metrics recorder
        // let _builder = PrometheusBuilder::new()
        //     .install()
        //     .expect("Failed to install Prometheus metrics recorder");

        Self {
            queries_total: Arc::new(RwLock::new(0)),
            queries_duration: Arc::new(RwLock::new(Vec::new())),
            rows_inserted_total: Arc::new(RwLock::new(0)),
            rows_read_total: Arc::new(RwLock::new(0)),
            active_connections: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn record_query(&self, duration_ms: f64) {
        let mut total = self.queries_total.write().await;
        *total += 1;
        drop(total);

        let mut durations = self.queries_duration.write().await;
        durations.push(duration_ms);
        if durations.len() > 1000 {
            durations.remove(0);
        }

        counter!("narayana_queries_total").increment(1);
        histogram!("narayana_query_duration_ms").record(duration_ms);
    }

    pub async fn record_insert(&self, rows: usize) {
        let mut total = self.rows_inserted_total.write().await;
        *total += rows as u64;
        counter!("narayana_rows_inserted_total").increment(rows as u64);
    }

    pub async fn record_read(&self, rows: usize) {
        let mut total = self.rows_read_total.write().await;
        *total += rows as u64;
        counter!("narayana_rows_read_total").increment(rows as u64);
    }

    pub async fn increment_connections(&self) {
        let mut conns = self.active_connections.write().await;
        *conns += 1;
        gauge!("narayana_active_connections").set(*conns as f64);
    }

    pub async fn decrement_connections(&self) {
        let mut conns = self.active_connections.write().await;
        if *conns > 0 {
            *conns -= 1;
        }
        gauge!("narayana_active_connections").set(*conns as f64);
    }

    pub async fn get_prometheus_metrics(&self) -> String {
        // In production, would use Prometheus encoder
        // For now, return basic metrics format
        format!(
            "# HELP narayana_queries_total Total number of queries\n\
             # TYPE narayana_queries_total counter\n\
             narayana_queries_total {}\n\
             # HELP narayana_rows_inserted_total Total rows inserted\n\
             # TYPE narayana_rows_inserted_total counter\n\
             narayana_rows_inserted_total {}\n",
            *self.queries_total.read().await,
            *self.rows_inserted_total.read().await
        )
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

