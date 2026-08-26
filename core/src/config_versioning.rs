//! Versioned trigger configuration, and notification of changes to it
//! (BE-022).
//!
//! Agents poll for configuration. Polling alone means an agent keeps acting on
//! a superseded threshold until its next tick, which is the staleness window
//! BE-022 is about. Two mechanisms close it, and they cover different failure
//! modes:
//!
//! **Push.** [`TriggerConfigVersioner::subscribe`] hands out a
//! [`broadcast`] receiver that fires the moment a version is added, so a
//! connected agent — or the WebSocket handler in [`crate::ws`], which uses the
//! same pattern — learns immediately rather than at its next poll.
//!
//! **Detection.** Push only helps agents that are connected. An agent that
//! reconnects, or misses an event because it lagged, still needs to be
//! detectable as stale. Every agent response carries its configuration
//! version in the [`CONFIG_VERSION_HEADER`] header, and
//! [`TriggerConfigVersioner::staleness_of`] turns that into an answer.
//!
//! The two together mean a stale agent is either corrected promptly or
//! visible, rather than silently wrong.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Header carrying the configuration version an agent is running.
///
/// Sent on agent responses so the backend can compare it against the current
/// version and detect an agent still acting on a superseded config.
pub const CONFIG_VERSION_HEADER: &str = "x-config-version";

/// Events buffered per subscriber before a slow one starts dropping them.
///
/// Matches [`crate::ws`]'s bus. A lagging subscriber loses individual events
/// but not correctness: it still learns the current version on its next poll,
/// and the version header still exposes it as stale in the meantime.
const CONFIG_BUS_CAPACITY: usize = 64;

/// How an agent's reported configuration version compares to the current one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigStaleness {
    /// The agent is on the current version.
    Current,

    /// The agent is behind by this many versions.
    Stale { behind_by: u64 },

    /// The agent reported a version ahead of anything issued — a rollback, or
    /// an agent talking to the wrong backend. Surfaced separately because
    /// treating it as "current" would hide a real misconfiguration.
    Ahead { ahead_by: u64 },

    /// No configuration exists for that trigger type yet.
    UnknownTrigger,
}

impl ConfigStaleness {
    /// Whether the agent needs to refresh.
    pub fn needs_refresh(&self) -> bool {
        matches!(self, ConfigStaleness::Stale { .. })
    }
}

/// Emitted whenever a new configuration version is recorded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigChangeEvent {
    pub trigger_type: String,
    pub version: u64,
    pub threshold: f64,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedTriggerConfig {
    pub version: u64,
    pub trigger_type: String,
    pub threshold: f64,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

pub struct TriggerConfigVersioner {
    versions: HashMap<String, Vec<VersionedTriggerConfig>>,
    changes: broadcast::Sender<ConfigChangeEvent>,
}

impl TriggerConfigVersioner {
    pub fn new() -> Self {
        let (changes, _) = broadcast::channel(CONFIG_BUS_CAPACITY);

        Self {
            versions: HashMap::new(),
            changes,
        }
    }

    /// Subscribe to configuration changes.
    ///
    /// The receiver fires as soon as a version is added, so a connected agent
    /// does not wait for its next poll. A subscriber that cannot keep up lags
    /// and skips events rather than blocking the publisher.
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChangeEvent> {
        self.changes.subscribe()
    }

    /// Record a new version and notify subscribers.
    ///
    /// Returns how many subscribers received the event. Zero is normal — it
    /// simply means nothing is currently listening, and the version is
    /// recorded either way.
    pub fn add_version(&mut self, trigger_type: &str, config: VersionedTriggerConfig) -> usize {
        let event = ConfigChangeEvent {
            trigger_type: trigger_type.to_string(),
            version: config.version,
            threshold: config.threshold,
            created_at: config.created_at,
            created_by: config.created_by.clone(),
        };

        self.versions
            .entry(trigger_type.to_string())
            .or_default()
            .push(config);

        // Publish only after the version is stored, so a subscriber that
        // immediately reads back the active config sees the one it was told
        // about rather than the previous one.
        self.changes.send(event).unwrap_or(0)
    }

    /// The current version for a trigger type, if any exists.
    pub fn current_version(&self, trigger_type: &str) -> Option<u64> {
        self.get_active_config(trigger_type).map(|c| c.version)
    }

    /// Compare a version reported by an agent against the current one.
    ///
    /// This is what the [`CONFIG_VERSION_HEADER`] on agent responses feeds.
    pub fn staleness_of(&self, trigger_type: &str, reported_version: u64) -> ConfigStaleness {
        let Some(current) = self.current_version(trigger_type) else {
            return ConfigStaleness::UnknownTrigger;
        };

        match reported_version.cmp(&current) {
            std::cmp::Ordering::Equal => ConfigStaleness::Current,
            std::cmp::Ordering::Less => ConfigStaleness::Stale {
                behind_by: current - reported_version,
            },
            std::cmp::Ordering::Greater => ConfigStaleness::Ahead {
                ahead_by: reported_version - current,
            },
        }
    }

    pub fn get_active_config(&self, trigger_type: &str) -> Option<&VersionedTriggerConfig> {
        self.versions.get(trigger_type)?.last()
    }

    pub fn get_config_at_time(
        &self,
        trigger_type: &str,
        time: DateTime<Utc>,
    ) -> Option<&VersionedTriggerConfig> {
        let configs = self.versions.get(trigger_type)?;
        configs.iter().filter(|c| c.created_at <= time).last()
    }

    pub fn snapshot_for_vault(
        &self,
        _vault_id: &str,
        trigger_type: &str,
    ) -> Option<VersionedTriggerConfig> {
        self.get_active_config(trigger_type).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(offset_secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000 + offset_secs, 0).expect("valid timestamp")
    }

    fn config(version: u64, threshold: f64, created_at: DateTime<Utc>) -> VersionedTriggerConfig {
        VersionedTriggerConfig {
            version,
            trigger_type: "drawdown".to_string(),
            threshold,
            created_at,
            created_by: "operator".to_string(),
        }
    }

    // ── Existing behaviour, previously untested ──────────────────────────

    #[test]
    fn the_active_config_is_the_most_recent_one() {
        let mut v = TriggerConfigVersioner::new();

        v.add_version("drawdown", config(1, 0.1, at(0)));
        v.add_version("drawdown", config(2, 0.2, at(10)));

        assert_eq!(v.get_active_config("drawdown").unwrap().version, 2);
        assert_eq!(v.current_version("drawdown"), Some(2));
    }

    #[test]
    fn an_unknown_trigger_has_no_active_config() {
        let v = TriggerConfigVersioner::new();

        assert!(v.get_active_config("nope").is_none());
        assert_eq!(v.current_version("nope"), None);
    }

    #[test]
    fn a_point_in_time_lookup_ignores_later_versions() {
        let mut v = TriggerConfigVersioner::new();

        v.add_version("drawdown", config(1, 0.1, at(0)));
        v.add_version("drawdown", config(2, 0.2, at(100)));

        // At t=50 only version 1 existed.
        assert_eq!(v.get_config_at_time("drawdown", at(50)).unwrap().version, 1);
        assert_eq!(
            v.get_config_at_time("drawdown", at(150)).unwrap().version,
            2
        );

        // Before anything was created there is no answer.
        assert!(v.get_config_at_time("drawdown", at(-1)).is_none());
    }

    #[test]
    fn trigger_types_are_independent() {
        let mut v = TriggerConfigVersioner::new();

        v.add_version("drawdown", config(1, 0.1, at(0)));
        v.add_version("volatility", config(7, 0.9, at(0)));

        assert_eq!(v.current_version("drawdown"), Some(1));
        assert_eq!(v.current_version("volatility"), Some(7));
    }

    // ── BE-022: push notification ────────────────────────────────────────

    /// The point of the issue: a change reaches a listener immediately
    /// instead of waiting for the agent's next poll.
    #[tokio::test]
    async fn a_subscriber_is_notified_of_a_change() {
        let mut v = TriggerConfigVersioner::new();
        let mut rx = v.subscribe();

        let delivered = v.add_version("drawdown", config(1, 0.25, at(0)));

        assert_eq!(delivered, 1);

        let event = rx.try_recv().expect("an event should be waiting");

        assert_eq!(
            event,
            ConfigChangeEvent {
                trigger_type: "drawdown".to_string(),
                version: 1,
                threshold: 0.25,
                created_at: at(0),
                created_by: "operator".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn every_subscriber_receives_every_change() {
        let mut v = TriggerConfigVersioner::new();
        let mut a = v.subscribe();
        let mut b = v.subscribe();

        assert_eq!(v.add_version("drawdown", config(1, 0.1, at(0))), 2);
        v.add_version("drawdown", config(2, 0.2, at(1)));

        for rx in [&mut a, &mut b] {
            assert_eq!(rx.try_recv().unwrap().version, 1);
            assert_eq!(rx.try_recv().unwrap().version, 2);
        }
    }

    /// Publishing with nobody listening is normal, not an error — the version
    /// must still be recorded.
    #[tokio::test]
    async fn a_change_with_no_subscribers_is_still_recorded() {
        let mut v = TriggerConfigVersioner::new();

        assert_eq!(v.add_version("drawdown", config(1, 0.1, at(0))), 0);
        assert_eq!(v.current_version("drawdown"), Some(1));
    }

    /// A subscriber that reacts to an event must see the version it was told
    /// about, which is why publication happens after the write.
    #[tokio::test]
    async fn the_version_is_readable_by_the_time_the_event_fires() {
        let mut v = TriggerConfigVersioner::new();
        let mut rx = v.subscribe();

        v.add_version("drawdown", config(3, 0.3, at(0)));

        let event = rx.try_recv().unwrap();

        assert_eq!(v.current_version(&event.trigger_type), Some(event.version));
    }

    #[tokio::test]
    async fn a_late_subscriber_misses_earlier_changes_but_receives_later_ones() {
        let mut v = TriggerConfigVersioner::new();

        v.add_version("drawdown", config(1, 0.1, at(0)));

        // Subscribing after the fact: broadcast has no replay, which is why
        // the version header exists as well.
        let mut rx = v.subscribe();

        assert!(rx.try_recv().is_err());

        v.add_version("drawdown", config(2, 0.2, at(1)));

        assert_eq!(rx.try_recv().unwrap().version, 2);
    }

    // ── BE-022: staleness detection ──────────────────────────────────────

    #[test]
    fn an_agent_on_the_current_version_is_not_stale() {
        let mut v = TriggerConfigVersioner::new();
        v.add_version("drawdown", config(5, 0.1, at(0)));

        let staleness = v.staleness_of("drawdown", 5);

        assert_eq!(staleness, ConfigStaleness::Current);
        assert!(!staleness.needs_refresh());
    }

    #[test]
    fn an_agent_behind_reports_how_far_behind() {
        let mut v = TriggerConfigVersioner::new();
        v.add_version("drawdown", config(1, 0.1, at(0)));
        v.add_version("drawdown", config(2, 0.2, at(1)));
        v.add_version("drawdown", config(5, 0.5, at(2)));

        let staleness = v.staleness_of("drawdown", 2);

        assert_eq!(staleness, ConfigStaleness::Stale { behind_by: 3 });
        assert!(staleness.needs_refresh());
    }

    /// An agent ahead of the backend is a rollback or a misrouted agent.
    /// Reporting it as current would hide a real misconfiguration.
    #[test]
    fn an_agent_ahead_is_reported_separately() {
        let mut v = TriggerConfigVersioner::new();
        v.add_version("drawdown", config(2, 0.1, at(0)));

        let staleness = v.staleness_of("drawdown", 9);

        assert_eq!(staleness, ConfigStaleness::Ahead { ahead_by: 7 });
        assert!(!staleness.needs_refresh());
    }

    #[test]
    fn staleness_of_an_unknown_trigger_is_distinguishable() {
        let v = TriggerConfigVersioner::new();

        assert_eq!(v.staleness_of("nope", 1), ConfigStaleness::UnknownTrigger);
    }

    #[test]
    fn the_version_header_name_is_stable() {
        // Agents depend on this spelling; changing it silently breaks
        // staleness detection for every deployed agent.
        assert_eq!(CONFIG_VERSION_HEADER, "x-config-version");
    }

    #[test]
    fn snapshot_for_vault_returns_the_active_config() {
        let mut v = TriggerConfigVersioner::new();
        v.add_version("drawdown", config(1, 0.1, at(0)));
        v.add_version("drawdown", config(2, 0.2, at(1)));

        let snapshot = v.snapshot_for_vault("vault-1", "drawdown").unwrap();

        assert_eq!(snapshot.version, 2);
    }
}
