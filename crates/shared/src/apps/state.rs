use tokio::sync::{broadcast, watch};

use crate::apps::{
    Alert, Profile,
    node::{GroupNode, Traffic},
};

/// Manages state synchronization between components using channels.
/// Provides a centralized hub for broadcasting updates on visibility, traffic metrics, groups, profiles, active profile, IP address, and alerts.
pub struct StateManager {
    /// Receiver and sender for visibility state updates (true/false).
    pub visibility_rx: watch::Receiver<bool>,
    visibility_tx: watch::Sender<bool>,

    /// Receiver and sender for traffic metric updates.
    pub traffic_rx: watch::Receiver<Traffic>,
    traffic_tx: watch::Sender<Traffic>,

    /// Receiver and sender for group node updates.
    pub groups_rx: watch::Receiver<Vec<GroupNode>>,
    groups_tx: watch::Sender<Vec<GroupNode>>,

    /// Receiver and sender for profile list updates.
    pub profiles_rx: watch::Receiver<Vec<Profile>>,
    profiles_tx: watch::Sender<Vec<Profile>>,

    /// Receiver and sender for active profile name updates.
    pub active_profile_rx: watch::Receiver<String>,
    active_profile_tx: watch::Sender<String>,

    /// Receiver and sender for IP address updates.
    pub ip_rx: watch::Receiver<String>,
    ip_tx: watch::Sender<String>,

    /// Sender for broadcasting alert notifications to all listeners.
    pub events: broadcast::Sender<Alert>,
}

impl StateManager {
    /// Initializes a new `StateManager` with default channels.
    /// Creates watch channels for visibility, traffic, groups, profiles, active profile, and IP address.
    /// Creates a broadcast channel with capacity of 100 for alerts.
    pub fn init() -> Self {
        let (visibility_tx, visibility_rx) = watch::channel(true);
        let (traffic_tx, traffic_rx) = watch::channel(Traffic::default());
        let (groups_tx, groups_rx) = watch::channel(Vec::new());
        let (profiles_tx, profiles_rx) = watch::channel(Vec::new());
        let (ip_tx, ip_rx) = watch::channel(String::new());
        let (active_profile_tx, active_profile_rx) = watch::channel(String::new());
        let (events, _) = broadcast::channel(100);

        Self {
            visibility_tx,
            visibility_rx,
            traffic_rx,
            traffic_tx,
            groups_rx,
            groups_tx,
            profiles_rx,
            profiles_tx,
            active_profile_tx,
            active_profile_rx,
            ip_tx,
            ip_rx,
            events,
        }
    }

    /// Updates the visibility state for all listeners.
    /// Sends a boolean value indicating whether the UI or component should be visible.
    /// The send operation is ignored if the channel is full.
    pub fn update_visibility(&self, visible: bool) {
        let _ = self.visibility_tx.send(visible);
    }

    /// Updates the traffic metrics for all listeners.
    /// Sends a new `Traffic` instance containing network performance data.
    /// The send operation is ignored if the channel is full.
    pub fn update_traffic(&self, metrics: Traffic) {
        let _ = self.traffic_tx.send(metrics);
    }

    /// Updates the list of group nodes for all listeners.
    /// Sends a new vector of `GroupNode` instances representing network groups.
    /// The send operation is ignored if the channel is full.
    pub fn update_groups(&self, groups: Vec<GroupNode>) {
        let _ = self.groups_tx.send(groups);
    }

    /// Updates the list of profiles for all listeners.
    /// Sends a new vector of `Profile` instances representing user or system profiles.
    /// The send operation is ignored if the channel is full.
    pub fn update_profiles(&self, profiles: Vec<Profile>) {
        let _ = self.profiles_tx.send(profiles);
    }

    /// Updates the active profile name for all listeners.
    /// Sends a string indicating the currently active profile.
    /// The send operation is ignored if the channel is full.
    pub fn update_active_profile(&self, profile: impl Into<String>) {
        let _ = self.active_profile_tx.send(profile.into());
    }

    /// Updates the IP address for all listeners.
    /// Sends a string containing the current IP address.
    /// The send operation is ignored if the channel is full.
    pub fn update_ip(&self, ip: impl Into<String>) {
        let _ = self.ip_tx.send(ip.into());
    }

    /// Notifies all listeners about an alert event.
    /// Sends an `Alert` instance to the broadcast channel, which will be delivered to all subscribers.
    /// The send operation is ignored if the channel is full.
    pub fn notify(&self, alert: Alert) {
        let _ = self.events.send(alert);
    }
}
