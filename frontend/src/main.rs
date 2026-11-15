mod app;
mod api;
mod buildings;
mod charts;
mod demo;
mod fmt;
mod log;
mod overview;
mod state;
mod tree;
mod ui;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
