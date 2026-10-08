#[cfg(target_arch = "wasm32")]
use catalog_ui::App;
#[cfg(target_arch = "wasm32")]
use leptos::mount::mount_to_body;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        mount_to_body(App);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("Ride Atlas UI binary runs as WebAssembly in the browser.");
    }
}
