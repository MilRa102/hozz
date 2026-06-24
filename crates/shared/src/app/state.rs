use tokio::sync::{broadcast, watch};

use crate::app::{
    alert::Alert,
    nodes::{GroupNode, Traffic},
    profile::Profile,
};

pub struct StateManager {
    pub visibility_rx: watch::Receiver<bool>,
    visibility_tx: watch::Sender<bool>,

    pub traffic_rx: watch::Receiver<Traffic>,
    traffic_tx: watch::Sender<Traffic>,

    pub groups_rx: watch::Receiver<Vec<GroupNode>>,
    groups_tx: watch::Sender<Vec<GroupNode>>,

    pub profiles_rx: watch::Receiver<Vec<Profile>>,
    profiles_tx: watch::Sender<Vec<Profile>>,

    pub active_profile_rx: watch::Receiver<String>,
    active_profile_tx: watch::Sender<String>,

    pub ip_rx: watch::Receiver<String>,
    ip_tx: watch::Sender<String>,

    pub events: broadcast::Sender<Alert>,
}

impl StateManager {
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

    pub fn update_visibility(&self, visible: bool) {
        let _ = self.visibility_tx.send(visible);
    }

    pub fn update_traffic(&self, metrics: Traffic) {
        let _ = self.traffic_tx.send(metrics);
    }

    pub fn update_groups(&self, groups: Vec<GroupNode>) {
        let _ = self.groups_tx.send(groups);
    }

    pub fn update_profiles(&self, profiles: Vec<Profile>) {
        let _ = self.profiles_tx.send(profiles);
    }

    pub fn update_active_profile(&self, profile: impl Into<String>) {
        let _ = self.active_profile_tx.send(profile.into());
    }

    pub fn update_ip(&self, ip: impl Into<String>) {
        let _ = self.ip_tx.send(ip.into());
    }

    pub fn notify(&self, alert: Alert) {
        let _ = self.events.send(alert);
    }
}
