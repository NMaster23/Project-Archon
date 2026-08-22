use tokio::io::AsyncWriteExt;
use wasmtime::{Config, Engine, Store, Result};
use wasmtime::component::{bindgen, Component, ResourceTable};
use turso::Builder;
use app_dirs2::{AppInfo, AppDataType, get_app_root};
use talos_core::TalosBus;
use wasmtime_wasi::{DirPerms, FilePerms, WasiCtx, WasiCtxBuilder, WasiView, WasiCtxView};

const APP_INFO: AppInfo = AppInfo {
    name: "Talos",
    author: "NMCreator",
};

bindgen!({
    path: "wit",
    world: "archon-extension",
    imports: { default: async },
    exports: { default: async }
});

pub struct PluginPermission {
    access_level: i32
}

pub struct PluginState {
    pub plugin_id: String,
    pub permissions: PluginPermission,
    pub db_pool: turso::Connection,
    pub http_client: reqwest::Client,
    pub event_sender: tokio::sync::mpsc::UnboundedSender<talos_core::TalosBus>,
    pub wasi_ctx: WasiCtx,
    pub table: ResourceTable,
}

impl WasiView for PluginState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table,
        }
    }
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

impl archon::plugin::host_capabilities::Host for PluginState {
    async fn emit_trace(&mut self, span_id: String, metrics_json: String) {
        let logs = format!("{},{},{}\n", self.plugin_id, span_id, metrics_json);
        if let Ok(mut file) = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("telemetry_traces.log")
            .await
            && let Err(e) = file.write_all(logs.as_bytes()).await {
                eprintln!("Couldn't write log: {}", e);
            }
    }
    async fn fetch_external(&mut self, url: String, _headers_json: String, _body: String) -> Result<String, String> {
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
    async fn vector_search(&mut self, query_vector: Vec<f32>, limit: u32) -> Result<Vec<String>, String> {
        if (self.permissions.access_level & 2) == 0 {
            return Err("Access level disabled".to_string());
        }
        let vector_bytes: Vec<u8> = query_vector.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        let mut rows = self.db_pool.query(
            "SELECT value FROM plugins ORDER BY vector_distance_cos(vector, vector32(?1)) LIMIT ?2",
            (vector_bytes, limit)
        ).await.map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            let val: String = row.get(0).map_err(|e| e.to_string())?;
            results.push(val);
        }
        Ok(results)
    }
    async fn schedule_task(&mut self, cron_expr: String, task_id: String) -> Result<bool, String> {
        let send_plugin = self.event_sender.send(TalosBus::PluginData {
            plugin_id: self.plugin_id.clone(),
            task_id,
            cron_expr,
        });
        if send_plugin.is_err() {
            return Err(send_plugin.unwrap_err().to_string());
        } else {
            Ok(true)
        }
    }
}

impl ArchonExtensionImports for PluginState {
    async fn log(&mut self, msg: String) {
        println!("Plugin {} Log: {}", self.plugin_id, msg);
    }
    async fn render_widget(&mut self, id: String, layout_json: String) {
        let sql = "INSERT OR REPLACE INTO plugin_widgets (plugin_id, widget_id, layout_json) VALUES (?, ?, ?)";
        if let Err(e) = self.db_pool.execute(sql, (self.plugin_id.clone(), id.clone(), layout_json.clone())).await {
            eprintln!("Couldn't insert plugin widget: {}", e);
        }
        let event = talos_core::TalosBus::RenderWidget {
            plugin_id: self.plugin_id.clone(),
            widget_id: id,
            layout_json,
        };
        if let Err(e) = self.event_sender.send(event) {
            eprintln!("Could not send event: {}", e);
        }
    }
}

pub async fn plugins(sender: tokio::sync::mpsc::UnboundedSender<talos_core::TalosBus>) -> Result<Vec<(String, ArchonExtension, Store<PluginState>)>> {
    let app_root = get_app_root(AppDataType::UserConfig, &APP_INFO)?;
    let plugin_dir = app_root.join("Plugins");
    tokio::fs::create_dir_all(&plugin_dir).await?;
    let plugin_db = plugin_dir.join("plugins.db");
    let db = Builder::new_local(plugin_db.to_str().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Plugin to string error"))?).build().await?;
    let conn = db.connect()?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS plugins (
        plugin_id TEXT,
        key TEXT,
        value TEXT,
        vector F32_BLOB(384),
        PRIMARY KEY (plugin_id, key)
        )",
        (),
    ).await?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS plugin_permissions (
            plugin_id TEXT PRIMARY KEY,
            access_level INTEGER
        )",
        ()
    ).await?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS plugin_widgets (
        plugin_id TEXT,
        widget_id TEXT,
        layout_json TEXT,
        PRIMARY KEY(plugin_id, widget_id))",
        ()
    ).await?;
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut plugins = Vec::new();
    let mut entries = tokio::fs::read_dir(plugin_dir.clone()).await?;
    let mut linker = wasmtime::component::Linker::<PluginState>::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    ArchonExtension::add_to_linker::<PluginState, wasmtime::component::HasSelf<PluginState>>(&mut linker, |state| state)?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wasm") {
            let mut wasi_builder = WasiCtxBuilder::new();
            wasi_builder.inherit_stdout().inherit_stderr().preopened_dir(&plugin_dir, "/workspace", DirPerms::all(), FilePerms::all())?;
            
            let wasi_ctx = wasi_builder.build();
            let table = ResourceTable::new();

            let component = match Component::from_file(&engine, &path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to load {:?}: {}", path, e);
                    continue;
                }
            };
            let plugin_data = PluginState {
                plugin_id: "".to_string(),
                permissions: PluginPermission { access_level: 0 },
                db_pool: conn.clone(),
                http_client: reqwest::Client::new(),
                event_sender: sender.clone(),
                wasi_ctx,
                table,
            };
            let mut store = Store::new(&engine, plugin_data);
            let instance = match ArchonExtension::instantiate_async(&mut store, &component, &linker).await {
                Ok(inst) => inst,
                Err(e) => {
                    eprintln!("Failed to instantiate {:?}: {}", path, e);
                    continue;
                }
            };
            if let Ok(manifest) = instance.call_manifest(&mut store).await
                && let Ok(manifest_json) = serde_json::from_str::<serde_json::Value>(&manifest)
                && let Some(plugin_id) = manifest_json["plugin_id"].as_str() {
                let plugin_id = plugin_id.to_string();
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO plugin_permissions (plugin_id, access_level) VALUES (?, ?)",
                    (plugin_id.clone(), 0)
                ).await;
                let mut rows = conn.query("SELECT access_level FROM plugin_permissions WHERE plugin_id = ?", (plugin_id.clone(),)).await?;
                if let Some(row) = rows.next().await? {
                    let access_level: i32 = row.get(0)?;
                    store.data_mut().permissions.access_level = access_level;
                }
                store.data_mut().plugin_id = plugin_id.clone();
                println!("Successfully loaded plugin: {}", plugin_id);
                plugins.push((plugin_id, instance, store));
            }
        }
    }
    Ok(plugins)
}

pub async fn install_plugin_url(plugin_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let app_root = get_app_root(AppDataType::UserConfig, &APP_INFO)?;
    let plugin_dir = app_root.join("Plugins");
    let fallback_plugin = format!("{}.wasm", plugin_url);
    tokio::fs::create_dir_all(&plugin_dir).await?;
    let parsed_url = reqwest::Url::parse(plugin_url)?;
    let file_name = &parsed_url
        .path_segments()
        .and_then(|mut s| s.next_back())
        .unwrap_or(fallback_plugin.as_str());
    let mut response = reqwest::get(parsed_url.clone()).await?;
    let mut file = tokio::fs::File::create(plugin_dir.join(file_name)).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }
    Ok(())
}