use std::{
    collections::HashMap,
    io::Write,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde_json::{Map, Value};
use ureq::{
    Body, Error, SendBody,
    http::{Request, Response},
    middleware::{Middleware, MiddlewareNext},
};

type Result<T, E = Error> = std::result::Result<T, E>;

const KEY_HEADERS: &str = "headers";

pub struct CacheMiddleware {
    dir: PathBuf,
    lockers: LockerMap,
}

impl CacheMiddleware {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_owned(),
            lockers: LockerMap::new(),
        }
    }

    fn cached(&self, key: &str) -> Option<Response<Body>> {
        let meta = cacache::metadata_sync(&self.dir, key).ok()??;
        let attrs = meta.metadata.as_object()?;
        let headers = attrs.get(KEY_HEADERS)?.as_object()?;

        let data = cacache::read_sync(&self.dir, key).ok()?;

        let mut response = Response::builder();

        for (key, value) in headers {
            if let Some(value) = value.as_str() {
                response = response.header(key, value);
            }
        }

        response.body(Body::builder().data(data)).ok()
    }

    fn save(&self, key: &str, response: &Response<&[u8]>) -> Option<()> {
        let mut headers = Map::new();
        for (key, value) in response.headers() {
            headers.insert(
                key.as_str().to_string(),
                Value::String(value.to_str().ok()?.to_string()),
            );
        }

        let data = response.body();

        let mut attrs = Map::new();
        attrs.insert(KEY_HEADERS.to_string(), Value::Object(headers));

        let mut cache = cacache::WriteOpts::new()
            .size(data.len())
            .metadata(Value::Object(attrs))
            .open_sync(&self.dir, key)
            .ok()?;

        cache.write_all(data).ok()?;
        cache.commit().ok()?;
        Some(())
    }
}

impl Middleware for CacheMiddleware {
    fn handle(
        &self,
        request: Request<SendBody>,
        next: MiddlewareNext,
    ) -> Result<Response<Body>, ureq::Error> {
        let key = request.uri().to_string();

        let locker = self.lockers.locker(key.clone());
        let _guard = locker.lock();

        if let Some(response) = self.cached(&key) {
            log::debug!("item found in cache: {key}");
            return Ok(response);
        }

        log::debug!("item not found in cache: {key}");

        let mut response = next.handle(request)?;
        if response.status() != 200 {
            return Ok(response);
        }

        if let Some(cc) = response.headers().get("Cache-Control") {
            let cc = cc.to_str().ok().unwrap_or_default();
            if cc.contains("no-store") || cc.contains("private") {
                log::warn!("cache control disables caching for {key}");
                return Ok(response);
            }
        }

        let body = response.body_mut();
        let data = body.read_to_vec()?;

        let mut bb = Body::builder();
        if let Some(charset) = body.charset() {
            bb = bb.charset(charset);
        }
        if let Some(mime_type) = body.mime_type() {
            bb = bb.mime_type(mime_type);
        }

        let response = Response::from_parts(response.into_parts().0, data.as_slice());

        if self.save(&key, &response).is_some() {
            log::debug!("item saved to cache: {key}");
        } else {
            log::warn!("failed to save item to cache: {key}");
        }

        let response = Response::from_parts(response.into_parts().0, bb.data(data));

        Ok(response)
    }
}

struct LockerMap(Mutex<HashMap<String, Arc<Mutex<()>>>>);

impl LockerMap {
    fn locker<'a>(&'a self, key: String) -> Locker<'a> {
        let mut lockers = self.0.lock().unwrap();
        Locker {
            map: self,
            key: key.clone(),
            lock: lockers
                .entry(key)
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone(),
        }
    }

    fn release(&self, key: &str) {
        let mut sync = self.0.lock().unwrap();
        sync.remove(key);
    }
}

impl LockerMap {
    fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

struct Locker<'a> {
    map: &'a LockerMap,
    key: String,
    lock: Arc<Mutex<()>>,
}

impl<'a> Drop for Locker<'a> {
    fn drop(&mut self) {
        self.map.release(&self.key);
    }
}

impl Deref for Locker<'_> {
    type Target = Mutex<()>;

    fn deref(&self) -> &Self::Target {
        &self.lock
    }
}
