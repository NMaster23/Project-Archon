use wasmtime::*;

pub async fn plugins() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;

    let wat = r#"
        (module
            (import "host" "host_func" (func $host_hello (param i32)))

            (func (export "hello")
                i32.const 3
                call $host_hello)
        )
    "#;
    let module = Module::new(&engine, wat)?;
    let mut linker = Linker::new(&engine);
    linker.func_wrap("host", "host_func", |mut _caller: Caller<'_, u32>, param: i32| {
        println!("Host function called from Wasm with: {}", param);
    })?;
    let mut store: Store<u32> = Store::new(&engine, 4);
    let instance = linker.instantiate_async(&mut store, &module).await?;
    let hello = instance.get_typed_func::<(), ()>(&mut store, "hello")?;
    hello.call_async(&mut store, ()).await?;
    Ok(())
}