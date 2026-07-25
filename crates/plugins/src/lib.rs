use wasmtime::{Config, Engine, Store, Result};
use wasmtime::component::{bindgen, Component};
use turso::Builder;

bindgen!({
    path: "wit",
    world: "archon-extension",
    async: true
});

pub struct PluginPermission {
    access_level: i32
}

pub struct PluginState {
    pub plugin_id: String,
    pub permissions: PluginPermission,
    pub db_pool: turso::Connection,
    pub http_client: reqwest::Client,
}

/*
access level bitflags (combinations)
0: Base Access (log, render_widget)
1: Network Access (fetch_external)
2: Database Read (db_get)
3: Network + DB Read (1 + 2)
4: Database Write (db_set)
5: Network + DB Write (1 + 4)
6: DB Read + DB Write (2 + 4)
7: Network + DB Read + DB Write (1 + 2 + 4)
8: Telemetry/System (emit_trace)
9: Network + Telemetry (1 + 8)
10: DB Read + Telemetry (2 + 8)
11: Unlimited
*/

#[async_trait::async_trait]
impl archon::plugin::host_capabilities::Host for PluginState {
    async fn emit_trace(&mut self, span_id: String, metrics_json: String) {
        println!("Emit trace: span-id {}, metrics-json {}", span_id, metrics_json);
    }
    async fn fetch_external(&mut self, url: String, headers_json: String, body: String) -> Result<String, String> {
        if (self.permissions.access_level & 1) == 0 {
            return Err(String::from("Access level disabled"));
        }
        let res = self.http_client.get(&url).send().await.map_err(|e| e.to_string())?;
        let body = res.text().await.map_err(|e| e.to_string())?;
        Ok(body)
    }
    async fn db_set(&mut self, key: String, value: String) -> Result<bool, String> {
        if (self.permissions.access_level & 4) == 0 {
            return Err("Access level disabled".to_string());
        }
        self.db_pool.execute("INSERT OR REPLACE INTO plugins (plugin_id, key, value) VALUES (?, ?, ?)", (self.plugin_id.clone(), key, value)).await.map_err(|e| e.to_string())?;
        Ok(true)
    }
    async fn db_get(&mut self, key: String) -> Result<Option<String>, String> {
        if (self.permissions.access_level & 2) == 0 {
            return Err(String::from("Access level disabled"));
        }
        let sql = "SELECT value FROM plugins WHERE plugin_id = ? AND key = ?";
        let mut rows = self.db_pool.query(sql, (self.plugin_id.clone(), key)).await.map_err(|e| e.to_string())?;
        match rows.next().await.map_err(|e| e.to_string())? {
            Some(row) => {
                let value: String = row.get(0).map_err(|e| e.to_string())?;
                Ok(Some(value))
            }
            None => Ok(None)
        }
    }
}

#[async_trait::async_trait]
impl ArchonExtensionImports for PluginState {
    async fn log(&mut self, msg: String) {
        println!("Plugin {} Log: {}", self.plugin_id, msg);
    }
    async fn render_widget(&mut self, id: String, layout_json: String) {
        
    }
}

pub async fn plugins() -> Result<()> {
    let db = Builder::new_local("app.db").build().await?;
    let conn = db.connect()?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS plugins (
        plugin_id TEXT,
        key TEXT,
        value TEXT,
        PRIMARY KEY (plugin_id, key)
        )",
        (),
    ).await.expect("Failed to create table plugins");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS plugin_permissions (
            plugin_id TEXT PRIMARY KEY,
            access_level INTEGER
        )",
        ()
    ).await.expect("Failed to create table plugin_permissions");
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut linker = wasmtime::component::Linker::<PluginState>::new(&engine);
    ArchonExtension::add_to_linker(&mut linker, |state| state)?;
    let component = Component::from_file(&engine, "plugin.wasm")?;
    let plugin_data = PluginState {
        plugin_id: "".to_string(),
        permissions: PluginPermission { access_level: 0 },
        db_pool: conn.clone(),
        http_client: reqwest::Client::new(),
    };
    let mut store = Store::new(&engine, plugin_data);
    let (instance, _) = ArchonExtension::instantiate_async(&mut store, &component, &linker).await?;
    let manifest = instance.call_manifest(&mut store).await?;
    let manifest_json: serde_json::Value = serde_json::from_str(&manifest)?;
    let plugin_id = manifest_json["plugin_id"].as_str().unwrap().to_string();
    conn.execute("INSERT OR REPLACE INTO plugin_permissions (plugin_id, access_level) VALUES (?, ?)", (plugin_id.clone(), 0)).await?;
    store.data_mut().plugin_id = plugin_id;
    Ok(())
}