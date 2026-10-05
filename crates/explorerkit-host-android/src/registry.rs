use crate::state::HostHandle;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};

static LIVE_HOSTS: LazyLock<Mutex<HashMap<usize, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static HOST_ACCESS_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static NEXT_HOST_GENERATION: AtomicU64 = AtomicU64::new(1);

// Controller handles that cross the JS/native boundary combine the host address with a
// generation. Every token lookup is revalidated against `LIVE_HOSTS` while `HOST_ACCESS_LOCK`
// is held so disposed hosts fail as expired handles even if an address is later reused.

pub(crate) fn register_live_host(handle: *mut HostHandle) {
    if handle.is_null() {
        return;
    }

    LIVE_HOSTS
        .lock()
        .expect("live host registry lock poisoned")
        .insert(
            handle as usize,
            NEXT_HOST_GENERATION.fetch_add(1, Ordering::Relaxed),
        );
}

#[cfg(test)]
pub(crate) fn unregister_live_host(handle: *mut HostHandle) {
    if handle.is_null() {
        return;
    }

    LIVE_HOSTS
        .lock()
        .expect("live host registry lock poisoned")
        .remove(&(handle as usize));
}

fn is_live_host(handle: *mut HostHandle) -> bool {
    live_host_generation(handle).is_some()
}

fn live_host_generation(handle: *mut HostHandle) -> Option<u64> {
    if handle.is_null() {
        return None;
    }
    LIVE_HOSTS
        .lock()
        .expect("live host registry lock poisoned")
        .get(&(handle as usize))
        .copied()
}

pub(crate) fn host_handle_token(handle: *mut HostHandle) -> Option<String> {
    if handle.is_null() {
        return None;
    }
    let _guard = HOST_ACCESS_LOCK.lock().expect("host access lock poisoned");
    let generation = live_host_generation(handle)?;

    Some(format!("{:x}:{:x}", handle as usize, generation))
}

fn host_from_token(token: &str) -> Result<(*mut HostHandle, u64), String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err("controller handle must not be empty".to_owned());
    }

    let Some((address, generation)) = trimmed.split_once(':') else {
        return Err("controller handle is invalid".to_owned());
    };

    let address = usize::from_str_radix(address, 16)
        .map_err(|_| "controller handle is invalid".to_owned())?;
    let generation = u64::from_str_radix(generation, 16)
        .map_err(|_| "controller handle is invalid".to_owned())?;
    Ok((address as *mut HostHandle, generation))
}

pub(crate) fn with_live_host<T>(
    host: *const HostHandle,
    f: impl FnOnce(&HostHandle) -> T,
) -> Option<T> {
    if host.is_null() {
        return None;
    }

    let _guard = HOST_ACCESS_LOCK.lock().expect("host access lock poisoned");
    let host = host.cast_mut();
    if !is_live_host(host) {
        return None;
    }

    Some(f(unsafe { &*host }))
}

pub(crate) fn with_live_host_mut<T>(
    host: *mut HostHandle,
    f: impl FnOnce(&mut HostHandle) -> T,
) -> Option<T> {
    if host.is_null() {
        return None;
    }

    let _guard = HOST_ACCESS_LOCK.lock().expect("host access lock poisoned");
    if !is_live_host(host) {
        return None;
    }

    Some(f(unsafe { &mut *host }))
}

pub(crate) fn with_token_host_mut<T>(
    token: &str,
    f: impl FnOnce(&mut HostHandle) -> T,
) -> Result<T, String> {
    // Resolve the JS-visible controller token back to a live host under the same access lock
    // that guards pointer liveness, so parsing and liveness validation stay coupled.
    let (host, generation) = host_from_token(token)?;
    if host.is_null() {
        return Err("controller handle is invalid".to_owned());
    }

    let _guard = HOST_ACCESS_LOCK.lock().expect("host access lock poisoned");
    if live_host_generation(host) != Some(generation) {
        return Err("controller handle is no longer valid".to_owned());
    }

    Ok(f(unsafe { &mut *host }))
}

pub(crate) fn free_live_host(host: *mut HostHandle) {
    if host.is_null() {
        return;
    }

    let _guard = HOST_ACCESS_LOCK.lock().expect("host access lock poisoned");
    let mut live_hosts = LIVE_HOSTS.lock().expect("live host registry lock poisoned");
    if live_hosts.remove(&(host as usize)).is_none() {
        return;
    }
    drop(live_hosts);

    drop(unsafe { Box::from_raw(host) });
}
