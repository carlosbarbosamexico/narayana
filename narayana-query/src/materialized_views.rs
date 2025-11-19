// Materialized views with automatic refresh - beyond ClickHouse

use narayana_core::{Error, Result, schema::Schema, types::TableId};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::plan::QueryPlan;

/// Materialized view definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedView {
    pub name: String,
    pub view_id: TableId,
    pub query_plan: QueryPlan,
    pub source_tables: Vec<TableId>,
    pub refresh_strategy: RefreshStrategy,
    pub last_refresh: u64,
    pub next_refresh: u64,
    pub incremental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefreshStrategy {
    Manual,
    OnDemand,
    Interval { seconds: u64 },
    OnCommit, // Refresh when source tables are updated
    Continuous, // Real-time updates
}

/// Materialized view manager
pub struct MaterializedViewManager {
    views: Arc<RwLock<HashMap<String, MaterializedView>>>,
    refresh_queue: Arc<crossbeam::queue::SegQueue<String>>,
}

impl MaterializedViewManager {
    pub fn new() -> Self {
        Self {
            views: Arc::new(RwLock::new(HashMap::new())),
            refresh_queue: Arc::new(crossbeam::queue::SegQueue::new()),
        }
    }

    /// Create materialized view
    pub fn create_view(
        &self,
        name: String,
        view_id: TableId,
        query_plan: QueryPlan,
        source_tables: Vec<TableId>,
        refresh_strategy: RefreshStrategy,
    ) -> Result<()> {
        let mut views = self.views.write();
        if views.contains_key(&name) {
            return Err(Error::Query(format!("Materialized view '{}' already exists", name)));
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let next_refresh = match &refresh_strategy {
            RefreshStrategy::Interval { seconds } => now + seconds,
            RefreshStrategy::OnDemand | RefreshStrategy::Manual => u64::MAX,
            RefreshStrategy::OnCommit | RefreshStrategy::Continuous => now,
        };

        let view = MaterializedView {
            name: name.clone(),
            view_id,
            query_plan,
            source_tables,
            refresh_strategy: refresh_strategy.clone(),
            last_refresh: 0,
            next_refresh,
            incremental: matches!(refresh_strategy, RefreshStrategy::Continuous | RefreshStrategy::OnCommit),
        };

        views.insert(name, view);
        Ok(())
    }

    /// Refresh materialized view
    pub async fn refresh_view(&self, name: &str) -> Result<()> {
        let mut views = self.views.write();
        if let Some(view) = views.get_mut(name) {
            // Execute query plan and update view
            // In production, would execute query and update table
            
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            view.last_refresh = now;
            
            // Schedule next refresh
            if let RefreshStrategy::Interval { seconds } = view.refresh_strategy {
                view.next_refresh = now + seconds;
            }
            
            Ok(())
        } else {
            Err(Error::Query(format!("Materialized view '{}' not found", name)))
        }
    }

    /// Incremental refresh (only update changed data)
    pub async fn incremental_refresh(&self, name: &str) -> Result<()> {
        let views = self.views.read();
        if let Some(view) = views.get(name) {
            if !view.incremental {
                return Err(Error::Query("View does not support incremental refresh".to_string()));
            }
            
            // In production, would compute delta and update only changed rows
            // This is much faster than full refresh
            
            Ok(())
        } else {
            Err(Error::Query(format!("Materialized view '{}' not found", name)))
        }
    }

    /// Auto-refresh views based on strategy
    pub async fn auto_refresh(&self) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Collect view names that need refreshing to avoid holding lock across await
        let views_to_refresh: Vec<String> = {
            let views = self.views.read();
            views.iter()
                .filter_map(|(name, view)| {
                    match &view.refresh_strategy {
                        RefreshStrategy::Interval { .. } if now >= view.next_refresh => Some(name.clone()),
                        RefreshStrategy::Continuous => Some(name.clone()),
                        _ => None,
                    }
                })
                .collect()
        };
        
        for name in views_to_refresh {
            let view_refresh = {
                let views = self.views.read();
                views.get(&name).and_then(|v| match &v.refresh_strategy {
                    RefreshStrategy::Interval { .. } => Some("full"),
                    RefreshStrategy::Continuous => Some("incremental"),
                    _ => None,
                })
            };
            
            if let Some(refresh_type) = view_refresh {
                match refresh_type {
                    "full" => self.refresh_view(&name).await?,
                    "incremental" => self.incremental_refresh(&name).await?,
                    _ => {}
                }
            }
        }
        
        Ok(())
    }

    /// Notify view of source table update (for OnCommit strategy)
    pub async fn notify_table_update(&self, table_id: TableId) -> Result<()> {
        // Collect view names that need refreshing to avoid holding lock across await
        let views_to_refresh: Vec<(String, &'static str)> = {
            let views = self.views.read();
            views.iter()
                .filter_map(|(name, view)| {
                    if view.source_tables.contains(&table_id) {
                        match view.refresh_strategy {
                            RefreshStrategy::OnCommit => Some((name.clone(), "full")),
                            RefreshStrategy::Continuous => Some((name.clone(), "incremental")),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect()
        };
        
        for (name, refresh_type) in views_to_refresh {
            match refresh_type {
                "full" => self.refresh_view(&name).await?,
                "incremental" => self.incremental_refresh(&name).await?,
                _ => {}
            }
        }
        
        Ok(())
    }

    /// Drop materialized view
    pub fn drop_view(&self, name: &str) -> Result<()> {
        let mut views = self.views.write();
        views.remove(name)
            .ok_or_else(|| Error::Query(format!("Materialized view '{}' not found", name)))?;
        Ok(())
    }

    /// List all materialized views
    pub fn list_views(&self) -> Vec<String> {
        let views = self.views.read();
        views.keys().cloned().collect()
    }
}

