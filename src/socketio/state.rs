use socketioxide::socket::Sid;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

pub type DevMap = HashMap<Sid, String>;
pub type SpeakerMap = HashMap<String, Sid>;
pub type UserMap = HashMap<Sid, String>;

#[derive(Default, Clone)]
pub struct OnlineDevs {
    onlinedevs: Arc<RwLock<DevMap>>,
    speakers: Arc<RwLock<SpeakerMap>>,
}

#[derive(Default, Clone)]
pub struct OnlineUsers {
    onlineusers: Arc<RwLock<UserMap>>,
}

impl OnlineDevs {
    pub async fn add(&self, sid: Sid, device: String) {
        let mut binding = self.onlinedevs.write().await;
        binding.entry(sid).or_insert(device);
    }

    pub async fn remove(&self, sid: &Sid) {
        let mut binding = self.onlinedevs.write().await;
        let _ = binding.remove(sid);
    }

    pub async fn speaker_on(&self, speakerid: Sid, device: &str) {
        let mut binding = self.speakers.write().await;
        binding.entry(device.to_owned()).or_insert(speakerid);
    }

    pub async fn speaker_off(&self, device: &str) {
        let mut binding = self.speakers.write().await;
        binding.remove(device);
    }

    pub async fn has_speaker(&self, device: &str) -> bool {
        let binding = self.speakers.read().await;
        binding.contains_key(device)
    }

    pub async fn getcamid(&self, device: &str) -> Option<Sid> {
        let devmap = self.onlinedevs.read().await;
        devmap.iter().find_map(|(key, val)| {
            if val == device {
                Some(key.to_owned())
            } else {
                None
            }
        })
    }

    pub async fn get(&self, sid: &Sid) -> Option<String> {
        self.onlinedevs.read().await.get(sid).cloned()
    }

    pub async fn val(&self) -> HashSet<String> {
        let devmap = self.onlinedevs.read().await;
        HashSet::from_iter(devmap.values().cloned())
    }
}

impl OnlineUsers {
    pub async fn add(&self, sid: Sid, user: String) {
        let mut binding = self.onlineusers.write().await;
        let _ = binding.entry(sid).or_insert(user);
    }

    pub async fn remove(&self, sid: &Sid) {
        let mut binding = self.onlineusers.write().await;
        let _ = binding.remove(sid);
    }

    pub async fn get(&self, sid: &Sid) -> Option<String> {
        self.onlineusers.read().await.get(sid).cloned()
    }

    pub async fn val(&self) -> HashSet<String> {
        let usermap = self.onlineusers.read().await;
        HashSet::from_iter(usermap.values().cloned())
    }

    pub async fn entries(&self) -> Vec<(String, String)> {
        let usermap = self.onlineusers.read().await;
        Vec::from_iter(
            usermap
                .iter()
                .map(|(key, val)| (key.to_string(), val.to_owned())),
        )
    }
}
