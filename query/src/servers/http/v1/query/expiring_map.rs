// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use common_base::base::tokio::task;
use common_base::base::tokio::time::sleep;
use common_base::infallible::RwLock;

use crate::servers::http::v1::query::expirable::Expirable;
use crate::servers::http::v1::query::expirable::ExpiringState;

// todo(youngsofun): use ExpiringMap for HttpQuery

struct MaybeExpiring<V>
where V: Expirable
{
    task: Option<task::JoinHandle<()>>,
    pub value: V,
}

impl<V> MaybeExpiring<V>
where V: Expirable
{
    pub fn on_expire(&mut self) {
        if let Some(t) = self.task.take() {
            t.abort()
        }
        self.value.on_expire();
    }
}

// on insert：start task
//   1. check V for expire
//   2. call remove if expire
// on remove:
//   1. call on_expire
//   2. Cancel task

pub struct ExpiringMap<K, V>
where V: Expirable
{
    map: Arc<RwLock<HashMap<K, MaybeExpiring<V>>>>,
}

async fn run_check<T: Expirable>(e: &T, max_idle: Duration) -> bool {
    loop {
        match e.expire_state() {
            ExpiringState::InUse(_) => sleep(max_idle).await,
            ExpiringState::Idle { idle_time } => {
                if idle_time > max_idle {
                    return true;
                } else {
                    sleep(max_idle - idle_time).await;
                    continue;
                }
            }
            ExpiringState::Aborted { need_cleanup } => return need_cleanup,
        }
    }
}

impl<K, V> Default for ExpiringMap<K, V>
where V: Expirable
{
    fn default() -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::default())),
        }
    }
}

impl<K, V> ExpiringMap<K, V>
where
    K: Hash + Eq,
    V: Expirable + Clone,
{
    pub fn insert(&mut self, k: K, v: V, max_idle_time: Option<Duration>)
    where
        K: Clone + Send + Sync + 'static,
        V: Send + Sync + 'static,
    {
        let mut map = self.map.write();
        let task = match max_idle_time {
            Some(d) => {
                let map_clone = self.map.clone();
                let v_clone = v.clone();
                let k_clone = k.clone();
                let task = task::spawn(async move {
                    if run_check(&v_clone, d).await {
                        Self::remove_inner(&map_clone, &k_clone);
                    }
                });
                Some(task)
            }
            None => None,
        };
        let i = MaybeExpiring { task, value: v };
        map.insert(k, i);
    }

    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let map = self.map.read();
        map.get(k).map(|i| &i.value).cloned()
    }

    pub fn remove<Q: ?Sized>(&mut self, k: &Q)
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        Self::remove_inner(&self.map, k)
    }

    fn remove_inner<Q: ?Sized>(map: &Arc<RwLock<HashMap<K, MaybeExpiring<V>>>>, k: &Q)
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let checker = {
            let mut map = map.write();
            map.remove(k)
        };
        if let Some(mut checker) = checker {
            checker.on_expire()
        }
    }
}
